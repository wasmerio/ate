package com.tokera.ate.io.kafka;

import com.tokera.ate.KafkaServer;
import com.tokera.ate.common.ApplicationConfigLoader;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.GenericPartitionKey;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.kafka.MessageSerializer;
import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.dao.msg.MessageData;
import com.tokera.ate.dao.msg.MessageType;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageBaseDto;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageMetaDto;
import com.tokera.ate.dto.msg.MessageSyncDto;
import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.DataPartitionChain;
import com.tokera.ate.io.repo.GenericPartitionBridge;
import com.tokera.ate.io.repo.IDataPartitionBridge;
import com.tokera.ate.io.repo.IDataTopicBridge;
import kafka.admin.AdminUtils;
import kafka.utils.ZKStringSerializer$;
import org.I0Itec.zkclient.ZkClient;
import org.I0Itec.zkclient.ZkConnection;
import org.apache.commons.lang3.time.StopWatch;
import org.apache.kafka.clients.consumer.ConsumerRecord;
import org.apache.kafka.clients.consumer.ConsumerRecords;
import org.apache.kafka.clients.consumer.KafkaConsumer;
import org.apache.kafka.clients.producer.KafkaProducer;
import org.apache.kafka.clients.producer.ProducerRecord;
import org.apache.kafka.common.TopicPartition;
import org.apache.kafka.common.errors.TopicExistsException;
import org.bouncycastle.crypto.InvalidCipherTextException;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.Response;
import java.io.IOException;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentSkipListSet;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;
import java.util.stream.Collectors;

/**
 * Represents the bridge of a particular Kafka topic
 */
public class KafkaTopicBridge implements Runnable, IDataTopicBridge {
    protected AteDelegate d = AteDelegate.get();
    protected LoggerHook LOG = new LoggerHook(KafkaTopicBridge.class);

    private final KafkaConfigTools m_config;
    private final DataPartitionType m_type;
    private final String m_bootstrapServers;
    private @MonotonicNonNull Thread thread;
    private AtomicInteger isInit = new AtomicInteger(0);
    private volatile int initLevel = -1;
    private volatile boolean isRunning = false;
    private volatile boolean isCreated = false;
    private AtomicInteger pollTimeout = new AtomicInteger(10);

    private @Nullable KafkaConsumer<String, MessageBase> consumer;
    private @Nullable KafkaProducer<String, MessageBase> producer;

    private final String topic;
    private final Map<Integer, GenericPartitionBridge> partitionBridges = new HashMap<>();

    private final Random rand = new Random();

    private ConcurrentHashMap<MessageSyncDto, Object> syncs = new ConcurrentHashMap<>();
    private ConcurrentHashMap<Integer, Boolean> isLoaded = new ConcurrentHashMap<>();
    private static ConcurrentSkipListSet<String> everCreated = new ConcurrentSkipListSet<>();

    public final static int maxTopics = 10000;
    public final static int maxPartitions = 200000;
    public final static int maxPartitionsPerTopic = maxPartitions / maxTopics;
    
    public KafkaTopicBridge(String topic, KafkaConfigTools config, DataPartitionType topicType, String bootstrapServers)
    {
        this.topic = topic;
        this.m_config = config;
        this.m_type = topicType;

        this.m_bootstrapServers = bootstrapServers;
    }

    public IDataPartitionBridge addKey(IPartitionKey key) {
        if (key.partitionTopic().equals(topic) ==false) {
            throw new WebApplicationException("Partition key does not match this topic.");
        }
        if (key.partitionIndex() >= maxPartitionsPerTopic) {
            throw new WebApplicationException("Partition index can not exceed the maximum of " + maxPartitionsPerTopic + " per topic.");
        }

        GenericPartitionBridge ret;
        int oldKeyCnt;
        synchronized (this.partitionBridges) {
            oldKeyCnt = partitionBridges.keySet().size();
            ret = partitionBridges.computeIfAbsent(key.partitionIndex(), i -> new GenericPartitionBridge(this, key, new DataPartitionChain(key)));
        }
        isInit.incrementAndGet();
        if (oldKeyCnt <= 0) {
            start();
        }
        return ret;
    }

    public boolean removeKey(IPartitionKey key) {
        if (this.topic.equals(key.partitionTopic()) == false) {
            return false;
        }

        int newKeyCnt;
        synchronized (this.partitionBridges) {
            boolean ret = partitionBridges.remove(key.partitionIndex()) != null;
            if (ret == false) return false;
            newKeyCnt = partitionBridges.size();
            isLoaded.remove((Integer)key.partitionIndex());
            isInit.incrementAndGet();
        }
        if (newKeyCnt <= 0) {
            stop();
        }
        return true;
    }

    @Override
    public Set<IPartitionKey> keys() {
        synchronized (this.partitionBridges) {
            return this.partitionBridges.keySet().stream().map(i -> new GenericPartitionKey(this.topic, i, m_type)).collect(Collectors.toSet());
        }
    }

    private void start() {
        synchronized (this) {
            if (this.thread == null) {
                this.thread = new Thread(this);
                this.thread.setDaemon(true);
            }

            this.isRunning = true;
            this.thread.start();
        }
    }
    
    private void stop() {
        synchronized (this) {
            isRunning = false;

            if (this.thread != null) {
                this.thread.interrupt();
            }
        }
    }

    private void touchConsumer() {
        if (this.consumer == null) {
            synchronized (this) {
                if (this.consumer == null) {
                    this.consumer = this.m_config.newConsumer(KafkaConfigTools.TopicRole.Consumer, KafkaConfigTools.TopicType.Dao, m_bootstrapServers);
                }
            }
        }
    }

    private void touchProducer() {
        if (this.producer == null) {
            synchronized (this) {
                if (this.producer == null) {
                    this.producer = this.m_config.newProducer(KafkaConfigTools.TopicRole.Producer, KafkaConfigTools.TopicType.Dao, m_bootstrapServers);
                }
            }
        }
    }

    private void initOrThrow()
    {
        if (isInit.get() == initLevel) {
            return;
        }

        synchronized (this)
        {
            int newLevel = isInit.get();
            if (newLevel == initLevel) {
                return;
            }

            // Load the producer (we will need it after the topic is created)
            touchProducer();

            // Create the topic if it doesn't already exist
            if (isCreated == false) {
                if (createTopic() == false) {
                    throw new WebApplicationException("Failed to create the new partitions.", Response.Status.INTERNAL_SERVER_ERROR);
                }
                isCreated = true;
            }

            // Load the consumer this is needed to load some of the data
            touchConsumer();

            // Take a snapshot of the keys we are adding
            Set<IPartitionKey> keys;
            synchronized (this.partitionBridges) {
                keys = this.partitionBridges.keySet().stream().map(i -> new GenericPartitionKey(this.topic, i, m_type)).collect(Collectors.toSet());
            }

            // Success now subscribe to these partitions
            KafkaConsumer<String, MessageBase> c = this.consumer;
            if (c == null) throw new WebApplicationException("Failed to initialize the Kafka consumer.", Response.Status.INTERNAL_SERVER_ERROR);
            KafkaProducer<String, MessageBase> p = this.producer;
            if (p == null) throw new WebApplicationException("Failed to initialize the Kafka producer.", Response.Status.INTERNAL_SERVER_ERROR);

            // Determine the new partitions and existing partition
            Set<Integer> existing = c.assignment().stream().map(a -> a.partition()).collect(Collectors.toSet());
            List<TopicPartition> partitions = keys.stream().map(k -> new TopicPartition(k.partitionTopic(), k.partitionIndex())).collect(Collectors.toList());
            List<TopicPartition> newPartitions = partitions.stream().filter(a -> existing.contains(a.partition()) == false).collect(Collectors.toList());

            // Assign the consumer to these partitions and go the start of newly assigned ones
            c.assign(partitions);
            if (newPartitions.size() != 0) {
                c.seekToBeginning(newPartitions);
            }

            // If we have the assigned partitions then we are done
            if (c.assignment().size() != keys.size()) {
                throw new WebApplicationException("Failed to assign the partitions.", Response.Status.REQUEST_TIMEOUT);
            }

            // Add the keys to the lookups that are waiting on the loaded event
            keys.forEach(k -> isLoaded.putIfAbsent((Integer)k.partitionIndex(), Boolean.FALSE));

            // Success
            initLevel = newLevel;
        }
    }

    /**
     * Initializes the partition by creating it
     */
    private boolean createTopic()
    {
        synchronized (this) {
            // If the topic has ever been created by this TokAPI then we dont attempt it again
            if (everCreated.contains(this.topic)) {
                return true;
            }

            // Load the properties for the zookeeper instance
            Properties props = d.bootstrapConfig.propertiesForKafka();

            // Add the bootstrap to the configuration file
            String zookeeperHosts = KafkaServer.getZooKeeperBootstrap();
            props.put("zookeeper.connect", zookeeperHosts);

            int connectionTimeOutInMs = 10000;
            Object connectionTimeOutInMsObj = MapTools.getOrNull(props, "zookeeper.connection.timeout.ms");
            if (connectionTimeOutInMsObj != null) {
                try {
                    connectionTimeOutInMs = Integer.parseInt(connectionTimeOutInMsObj.toString());
                } catch (NumberFormatException ex) {
                }
            }
            int sessionTimeOutInMs = 10000;

            int numOfReplicas = 1;
            Object numOfReplicasObj = MapTools.getOrNull(props, "default.replication.factor");
            if (numOfReplicasObj != null) {
                try {
                    numOfReplicas = Integer.parseInt(numOfReplicasObj.toString());
                } catch (NumberFormatException ex) {
                }
            }

            ZkClient client = new ZkClient(zookeeperHosts, sessionTimeOutInMs, connectionTimeOutInMs, ZKStringSerializer$.MODULE$);
            kafka.utils.ZkUtils utils = new kafka.utils.ZkUtils(client, new ZkConnection(zookeeperHosts), false);

            // If it already exists the nwe are done
            if (AdminUtils.topicExists(utils, this.topic)) {
                everCreated.add(this.topic);
                return true;
            }

            // Load the topic properties depending on the need
            String topicPropsName;
            switch (this.m_type) {
                default:
                case Dao:
                    topicPropsName = d.bootstrapConfig.getPropertiesFileTopicDao();
                    break;
                case Io:
                    topicPropsName = d.bootstrapConfig.getPropertiesFileTopicIo();
                    break;
                case Publish:
                    topicPropsName = d.bootstrapConfig.getPropertiesFileTopicPublish();
                    break;
            }
            Properties topicProps = ApplicationConfigLoader.getInstance().getPropertiesByName(topicPropsName);

            // Enter a retry loop with exponential backoff
            int delayMs = 100;
            for (int n = 0;; n++)
            {
                if (topicProps != null) {
                    // Create the topic
                    try {
                        AdminUtils.createTopic(utils, this.topic, maxPartitionsPerTopic, numOfReplicas, topicProps, kafka.admin.RackAwareMode.Disabled$.MODULE$);
                        for (int p = 0; p < maxPartitionsPerTopic; p++) {
                            isLoaded.put((Integer) p, true);
                        }
                        everCreated.add(this.topic);
                        return true;
                    } catch (TopicExistsException ex) {
                        everCreated.add(this.topic);
                        return true;
                    } catch (Throwable ex) {
                        if (n >= 7) {
                            LOG.warn(ex);
                            return false;
                        }
                        try {
                            Thread.sleep(delayMs);
                        } catch (InterruptedException e) {
                            LOG.warn(ex);
                            return false;
                        }
                        delayMs *= 2;
                        continue;
                    }
                }
            }
        }
    }
    
    @Override
    public void run() {
        Long errorWaitTime = 500L;
        
        // Enter the main processing loop
        StopWatch timer = new StopWatch();
        timer.start();
        while (isRunning) {
            try {
                // Initialize or throw an exception
                initOrThrow();
        
                // Perform a poll of all the data for topics
                int numRecords = poll();
                
                // When we've reached the end of the records then this is an indicator that everything has been
                // loaded for these key sets
                if (numRecords <= 0) {
                    for (Integer n : isLoaded.keySet()) {
                        if (isLoaded.get(n) == false) {
                            isLoaded.put(n, true);
                        }
                    }
                }
                
                errorWaitTime = 500L;
            } catch (Throwable ex) {
                if (ex instanceof InterruptedException) {
                    if (isRunning == false) break;
                    dispose();
                    throw ex;
                }
                LOG.error(ex);
                try {
                    Thread.sleep(errorWaitTime);
                } catch (InterruptedException ex1) {
                    if (isRunning == false) break;
                    LOG.warn(ex1);
                    continue;
                }
                errorWaitTime *= 2L;
                if (errorWaitTime > 4000L) {
                    errorWaitTime = 4000L;
                }
            }
        }
    }
    
    private void dispose()
    {
        synchronized (this) {
            if (this.consumer != null) {
                this.consumer.close();
                this.consumer = null;
            }
            if (this.producer != null) {
                this.producer.close();
                this.producer = null;
            }
        }

        isInit.set(0);
        isLoaded.clear();
        synchronized (this.partitionBridges) {
            this.partitionBridges.clear();
        }
    }
    
    private int poll()
    {
        // While we are waiting the poll time is high otherwise it slows down
        int pollTimeout = this.pollTimeout.getAndAdd(10);
        if (pollTimeout > 100) pollTimeout = 100;
        if (this.isLoaded.values().stream().anyMatch(v -> v == false)) {
            pollTimeout = 10;
        }

        // Process all the records using this polling timeout
        int foundRecords = 0;
        int emptyCount = 0;
        while (true)
        {
            KafkaConsumer<String, MessageBase> c = this.consumer;
            if (c == null) {
                dispose();
                break;
            }

            final ConsumerRecords<String, MessageBase> consumerRecords =
                c.poll(pollTimeout);

            if (consumerRecords.isEmpty() == true) {
                emptyCount++;
                if (emptyCount > 10) {
                    break;
                }
            } else {
                foundRecords += consumerRecords.count();
                emptyCount = 0;
            }

            synchronized (this.partitionBridges) {
                consumerRecords.forEach(record -> {
                    if (record.topic().equals(topic) == true)
                    {
                        // Now find the bridge and send the message to it
                        IDataPartitionBridge partitionBridge = MapTools.getOrNull(this.partitionBridges, record.partition());
                        if (partitionBridge != null) {
                            d.debugLogging.logKafkaRecord(record);

                            // Now process the message itself
                            MessageMetaDto meta = new MessageMetaDto(
                                    record.partition(),
                                    record.offset(),
                                    record.timestamp());

                            if (record.value().msgType() == MessageType.MessageSync) {
                                processSync(new MessageSyncDto(record.value()), LOG);
                                return;
                            }
                            try {
                                boolean loaded = isLoaded.getOrDefault((Integer)record.partition(), false);
                                partitionBridge.chain().rcv(record.value(), meta, loaded, LOG);
                            } catch (IOException | InvalidCipherTextException ex) {
                                LOG.warn(ex);
                            }
                        }
                    }
                });
            }
        }
        
        return foundRecords;
    }

    public MessageSyncDto startSync(IPartitionKey key) {
        MessageSyncDto sync = new MessageSyncDto(
                rand.nextLong(),
                rand.nextLong());
        startSync(key, sync, new Object());
        return sync;
    }

    public MessageSyncDto startSync(IPartitionKey key, MessageSyncDto sync) {
        sync = new MessageSyncDto(sync);
        startSync(key, sync, new Object());
        return sync;
    }

    private void startSync(IPartitionKey key, MessageSyncDto sync, Object waitOn) {
        syncs.put(sync, waitOn);

        this.send(key, sync);

        d.debugLogging.logSyncStart(sync);
    }

    public boolean hasFinishSync(IPartitionKey key, MessageSyncDto sync) {
        return syncs.containsKey(sync) == false;
    }

    public boolean finishSync(IPartitionKey key, MessageSyncDto sync) {
        return finishSync(key, sync, 60);
    }

    public boolean finishSync(IPartitionKey key, MessageSyncDto sync, int timeout) {
        Object wait = MapTools.getOrNull(this.syncs, sync);
        if (wait == null) return true;

        synchronized (wait) {
            if (syncs.containsKey(sync) == false) {
                return true;
            }

            try {
                wait.wait(timeout);
                return hasFinishSync(key, sync);
            } catch (InterruptedException e) {
                return false;
            } finally {
                syncs.remove(sync);
            }
        }
    }

    public boolean sync(IPartitionKey key) {
        return sync(key, 60000);
    }

    public boolean sync(IPartitionKey key, int timeout) {

        Object wait = new Object();
        synchronized (wait)
        {
            MessageSyncDto sync = new MessageSyncDto(
                    rand.nextLong(),
                    rand.nextLong());
            startSync(key, sync, wait);

            try {
                wait.wait(timeout);
                d.debugLogging.logSyncWake(sync);
                return hasFinishSync(key, sync);
            } catch (InterruptedException e) {
                return false;
            } finally {
                syncs.remove(sync);
            }
        }
    }

    private void processSync(MessageSyncDto sync, LoggerHook LOG)
    {
        d.debugLogging.logReceive(sync);

        Object wait = syncs.remove(sync);
        if (wait == null) {
            d.debugLogging.logSyncMiss(sync);
            return;
        }

        synchronized (wait) {
            d.debugLogging.logSyncFinish(sync);
            wait.notifyAll();
        }
    }

    public void waitTillLoaded(IPartitionKey key)  {
        boolean sentSync = false;
        if (isLoaded.getOrDefault((Integer)key.partitionIndex(), false) == false) {
            StopWatch waitTime = new StopWatch();
            waitTime.start();
            while (isLoaded.getOrDefault((Integer)key.partitionIndex(), false) == false) {
                if (waitTime.getTime() > 5000L) {
                    if (sentSync == false) {
                        this.send(key, new MessageSyncDto(0, 0), false);
                        sentSync = true;
                    }
                }
                if (waitTime.getTime() > 20000L) {
                    StringBuilder sb  = new StringBuilder();
                    synchronized (this.partitionBridges) {
                        this.partitionBridges.keySet().forEach(i -> {
                            if (sb.length() > 0) sb.append(",");
                            sb.append(i);
                        });
                    }
                    throw new RuntimeException("Busy loading data partition [topic=" + this.topic + ", partitions=" + sb.toString() + ", isCreated=" + this.isCreated + "]");
                }
                try {
                    Thread.sleep(50);
                } catch (InterruptedException ex) {
                    break;
                }
            }
        }
    }

    public void send(IPartitionKey key, MessageBaseDto msg)
    {
        send(key, msg, true);
    }
    
    public void send(IPartitionKey key, MessageBaseDto msg, boolean shouldWaitForLoaded)
    {
        // Send the message do Kafka
        ProducerRecord<String, MessageBase> record = new ProducerRecord<>(key.partitionTopic(), key.partitionIndex(), MessageSerializer.getKey(msg), msg.createBaseFlatBuffer());
        if (shouldWaitForLoaded == true) {
            waitTillLoaded(key);
        }
        
        // Send the record to Kafka
        if (producer != null) {
            producer.send(record);
        }

        d.debugLogging.logKafkaSend(record, msg);
    }
   
    public @Nullable MessageDataDto getVersion(PUUID id, MessageMetaDto meta) {
        TopicPartition tp = new TopicPartition(id.partitionTopic(), id.partitionIndex());
        
        List<TopicPartition> tps = new LinkedList<>();
        tps.add(tp);
        
        KafkaConsumer<String, MessageBase> onceConsumer = m_config.newConsumer(KafkaConfigTools.TopicRole.Consumer, KafkaConfigTools.TopicType.Dao, m_bootstrapServers);
        onceConsumer.assign(tps);
        onceConsumer.seek(tp, meta.getOffset());
        
        final ConsumerRecords<String, MessageBase> consumerRecords = onceConsumer.poll(5000);
        if (consumerRecords.isEmpty()) return null;
        
        for (ConsumerRecord<String, MessageBase> msg : consumerRecords) {
            if (msg.partition() == meta.getPartition() &&
                msg.offset() == meta.getOffset())
            {
                if (msg.value().msgType() == MessageType.MessageData) {
                    MessageData data = (MessageData)msg.value().msg(new MessageData());
                    if (data == null) return null;
                    return new MessageDataDto(data);
                }
            }
        }
        
        return null;
    }
}
