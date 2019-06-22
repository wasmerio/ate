package com.tokera.ate.io.kafka;

import com.tokera.ate.KafkaServer;
import com.tokera.ate.dao.kafka.MessageSerializer;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.DataPartitionChain;
import com.tokera.ate.io.repo.IDataPartitionBridge;
import com.tokera.ate.common.ApplicationConfigLoader;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.msg.*;
import com.tokera.ate.dto.msg.MessageBaseDto;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageMetaDto;
import com.tokera.ate.dto.msg.MessageSyncDto;
import com.tokera.ate.enumerations.DataPartitionType;
import java.io.IOException;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.stream.Collectors;

import org.apache.commons.lang.time.StopWatch;
import org.apache.kafka.clients.consumer.ConsumerRecord;
import org.apache.kafka.clients.consumer.ConsumerRecords;
import org.apache.kafka.clients.consumer.KafkaConsumer;
import org.apache.kafka.clients.producer.KafkaProducer;
import org.apache.kafka.clients.producer.ProducerRecord;
import org.apache.kafka.common.PartitionInfo;
import org.apache.kafka.common.TopicPartition;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.bouncycastle.crypto.InvalidCipherTextException;
import kafka.admin.AdminUtils;
import org.I0Itec.zkclient.ZkClient;
import org.I0Itec.zkclient.ZkConnection;
import kafka.utils.ZKStringSerializer$;
import org.apache.kafka.common.errors.TopicExistsException;

/**
 * Represents the bridge of a particular Kafka topic
 */
public class KafkaPartitionBridge implements Runnable, IDataPartitionBridge {

    protected AteDelegate d = AteDelegate.get();
    protected LoggerHook LOG = new LoggerHook(KafkaPartitionBridge.class);

    private final IPartitionKey m_key;
    private final DataPartitionChain m_chain;
    private final KafkaConfigTools m_config;
    private final DataPartitionType m_type;
    private final String m_bootstrapServers;
    private @MonotonicNonNull Thread thread;
    private volatile boolean isRunning = true;
    private volatile boolean isLoaded = false;
    private volatile boolean isEthereal = false;
    private volatile boolean isCreated = false;
    private volatile boolean hasLoadingMessages = false;

    private @Nullable KafkaConsumer<String, MessageBase> consumer;
    private @Nullable KafkaProducer<String, MessageBase> producer;

    private List<TopicPartition> partitions = new LinkedList<>();

    private final Random rand = new Random();
    private Map<MessageSyncDto, Object> syncs = new ConcurrentHashMap<>();
    
    public KafkaPartitionBridge(IPartitionKey key, DataPartitionChain chain, KafkaConfigTools config, DataPartitionType topicType, String bootstrapServers)
    {
        this.m_key = key;
        this.m_chain = chain;
        this.m_config = config;
        this.m_type = topicType;

        this.m_bootstrapServers = bootstrapServers;
    }
    
    public void start() {
        if (this.thread == null) {
            this.thread = new Thread(this);
            this.thread.setDaemon(true);
        }

        this.isRunning = true;
        this.thread.start();
    }
    
    public void stop() {
        isRunning = false;

        if (this.thread != null) {
            this.thread.interrupt();
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
        
                // Load the consumer and producer
                if (this.consumer == null) {
                    this.consumer = this.m_config.newConsumer(KafkaConfigTools.TopicRole.Consumer, KafkaConfigTools.TopicType.Dao, m_bootstrapServers);
                }
                
                // We only create the producer once things have got going
                if (this.producer == null) {
                    this.producer = this.m_config.newProducer(KafkaConfigTools.TopicRole.Producer, KafkaConfigTools.TopicType.Dao, m_bootstrapServers);
                }
        
                // Perform a poll of all the data for topics
                int numRecords = poll();
                
                // Check if records have been loaded or enough time has passed
                // that we judge it is an empty partition
                if (isLoaded == false) {
                    if (hasLoadingMessages) {
                        if (numRecords <= 0) isLoaded = true;
                        else if (timer.getTime() > 15000) isLoaded = true;
                    } else {
                        if (numRecords > 0 || this.isCreated) hasLoadingMessages = true;
                        else if (timer.getTime() > 15000) isLoaded = true;
                    }
                }
                
                errorWaitTime = 500L;
            } catch (Throwable ex) {
                if (ex instanceof InterruptedException) {
                    dispose();
                    throw ex;
                }
                LOG.error(ex);
                try {
                    Thread.sleep(errorWaitTime);
                } catch (InterruptedException ex1) {
                    LOG.warn(ex1);
                    return;
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
        if (this.consumer != null) {
            this.consumer.close();
            this.consumer = null;
        }
        if (this.producer != null) {
            this.producer.close();
            this.producer = null;
        }
        this.partitions.clear();

        isLoaded = false;
        hasLoadingMessages = false;
    }
    
    private int poll()
    {
        int foundRecords = 0;
        int emptyCount = 0;
        while (true)
        {
            if (touchConsumer() == false)
            {
                dispose();
                break;
            }

            KafkaConsumer<String, MessageBase> c = this.consumer;
            if (c == null) {
                dispose();
                break;
            }

            final ConsumerRecords<String, MessageBase> consumerRecords =
                c.poll(100);

            if (consumerRecords.isEmpty() == true) {
                emptyCount++;
                if (emptyCount > 10) {
                    break;
                }
            } else {
                foundRecords += consumerRecords.count();
                emptyCount = 0;
            }

            consumerRecords.forEach(record -> {
                if (record.topic().equals(m_key.partitionTopic()) == true &&
                    record.partition() == m_key.partitionIndex())
                {
                    d.debugLogging.logKafkaRecord(record, LOG);
                    
                    MessageMetaDto meta = new MessageMetaDto(
                            record.partition(),
                            record.offset(),
                            record.timestamp());

                    if (record.value().msgType() == MessageType.MessageSync) {
                        processSync(new MessageSyncDto(record.value()), LOG);
                        return;
                    }

                    try {
                        m_chain.rcv(record.value(), meta, LOG);
                    } catch (IOException | InvalidCipherTextException ex) {
                        LOG.warn(ex);
                    }
                }
            });
        }
        
        return foundRecords;
    }

    public MessageSyncDto startSync() {
        return startSync(new Object());
    }

    private MessageSyncDto startSync(Object waitOn) {
        MessageSyncDto sync = new MessageSyncDto(
                rand.nextLong(),
                rand.nextLong());

        syncs.put(sync, waitOn);

        this.send(sync);

        d.debugLogging.logSyncStart(sync, LOG);
        return sync;
    }

    public boolean hasFinishSync(MessageSyncDto sync) {
        return syncs.containsKey(sync) == false;
    }

    public boolean finishSync(MessageSyncDto sync) {
        return finishSync(sync, 60);
    }

    public boolean finishSync(MessageSyncDto sync, int timeout) {
        Object wait = MapTools.getOrNull(this.syncs, sync);
        if (wait == null) return true;

        synchronized (wait) {
            if (syncs.containsKey(sync) == false) {
                return true;
            }

            try {
                wait.wait(timeout);
                return hasFinishSync(sync);
            } catch (InterruptedException e) {
                return false;
            } finally {
                syncs.remove(sync);
            }
        }
    }

    public boolean sync() {
        return sync(60000);
    }

    public boolean sync(int timeout) {

        Object wait = new Object();
        synchronized (wait)
        {
            MessageSyncDto sync = startSync(wait);

            try {
                wait.wait(timeout);
                d.debugLogging.logSyncWake(sync, LOG);
                return hasFinishSync(sync);
            } catch (InterruptedException e) {
                return false;
            } finally {
                syncs.remove(sync);
            }
        }
    }

    private void processSync(MessageSyncDto sync, LoggerHook LOG)
    {
        d.debugLogging.logReceive(sync, LOG);

        Object wait = syncs.remove(sync);
        if (wait == null) {
            d.debugLogging.logSyncMiss(sync, LOG);
            return;
        }

        synchronized (wait) {
            d.debugLogging.logSyncFinish(sync, LOG);
            wait.notifyAll();
        }
    }

    public void waitTillLoaded()  {
        if (isLoaded == false) {
            StopWatch waitTime = new StopWatch();
            waitTime.start();
            while (isLoaded == false) {
                if (isEthereal) return;
                if (waitTime.getTime() > 20000L) {
                    throw new RuntimeException("Busy loading data partition [" + m_chain.getPartitionKeyStringValue() + "]");
                }
                try {
                    Thread.sleep(50);
                } catch (InterruptedException ex) {
                    break;
                }
            }
        }
    }

    @SuppressWarnings({"return.type.incompatible"})       // This is a fix for the consumer which can actually return null in this instance
    private static @Nullable List<PartitionInfo> partitionsForOrNull(KafkaConsumer<String, MessageBase> consumer, String topic) {
        return consumer.partitionsFor(topic);
    }
    
    private boolean touchConsumer()
    {
        KafkaConsumer<String, MessageBase> c = this.consumer;
        if (c == null) return false;

        // If the consumer is not yet assigned to partitions then assign it
        if (c.assignment().size() <= 0)
        {
            // Configure the consumers
            if (this.partitions.isEmpty())
            {
                // Attempt to load the parititons for this topic in a loop and while
                // they are not loaded (because the topic doesnt exist) switch to an
                // ethereal state
                List<PartitionInfo> parts = null;
                if (parts == null) {
                    while (true) {
                        parts = KafkaPartitionBridge.partitionsForOrNull(c, this.m_key.partitionTopic());
                        if (parts != null &&
                            parts.size() > 0)
                        {
                            isEthereal = false;
                            break;
                        }

                        isEthereal = true;
                        try {
                            Thread.sleep(200);
                        } catch (InterruptedException ex1) {
                            LOG.warn(ex1);
                            return false;
                        }
                    }
                }
                
                if (parts == null) return false;
                this.partitions = parts.stream()
                        .filter(i -> i.partition() == m_key.partitionIndex())
                        .map(i -> new TopicPartition(i.topic(), i.partition()))
                        .collect(Collectors.toList());
            }

            c.assign(this.partitions);
            c.seekToBeginning(this.partitions);
        }
        
         return true;
    }
    
    public void touchProducer()
    {
        // The producer must also could the consumer otherwise it will wait
        // until it times out
        // WARNING: Can not touch the consumer while poll is going on or we hit a multithreading issue
        //          as the kafka consumer is not threadsafe
        //touchConsumer();

        // Wait for the topic to go into an Ethereal status or load
        StopWatch waitTime = new StopWatch();
        waitTime.start();

        waitTillLoaded();

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

        int numOfPartitions = 1;
        Object numOfPartitionsObj = MapTools.getOrNull(props, "num.partitions");
        if (numOfPartitionsObj != null) {
            try {
                numOfPartitions = Integer.parseInt(numOfPartitionsObj.toString());
            } catch (NumberFormatException ex) {
            }
        }

        int numOfReplicas = 2;
        Object numOfReplicasObj = MapTools.getOrNull(props, "default.replication.factor");
        if (numOfReplicasObj != null) {
            try {
                numOfReplicas = Integer.parseInt(numOfReplicasObj.toString());
            } catch (NumberFormatException ex) {
            }
        }

        ZkClient client = new ZkClient(zookeeperHosts, sessionTimeOutInMs, connectionTimeOutInMs, ZKStringSerializer$.MODULE$);
        kafka.utils.ZkUtils utils = new kafka.utils.ZkUtils(client, new ZkConnection(zookeeperHosts), false);

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
        if (topicProps != null) {
            // Create the topic
            try {
                int maxPartitionsPerTopic = AteDelegate.get().io.partitionKeyMapper().maxPartitionsPerTopic();
                AdminUtils.createTopic(utils, this.m_key.partitionTopic(), maxPartitionsPerTopic, numOfReplicas, topicProps, kafka.admin.RackAwareMode.Disabled$.MODULE$);
                this.isCreated = true;
            } catch (TopicExistsException ex) {
                this.isCreated = true;
            }
        }
        this.isEthereal = false;

        // Wait for the topic to come online
        while (isLoaded == false) {
            if (waitTime.getTime() > 20000L) {
                throw new RuntimeException("Busy while creating data topic [" + m_key.partitionTopic() + "]");
            }
            try {
                Thread.sleep(50);
            } catch (InterruptedException ex) {
                break;
            }
        }
    }
    
    public void send(MessageBaseDto msg)
    {
        // Send the message do Kafka
        ProducerRecord<String, MessageBase> record = new ProducerRecord<>(this.m_key.partitionTopic(), this.m_key.partitionIndex(), MessageSerializer.getKey(msg), msg.createBaseFlatBuffer());
        waitTillLoaded();
        
        // If we are Ethereal then we should attempt to create the topic and
        // then wait for it to be loaded
        if (this.isEthereal == true) {
            touchProducer();
        }
        
        // Send the record to Kafka
        if (producer != null) {
            producer.send(record);
        }

        d.debugLogging.logKafkaSend(record, msg, LOG);
    }
   
    public @Nullable MessageDataDto getVersion(UUID id, MessageMetaDto meta) {
        TopicPartition tp = new TopicPartition(this.m_key.partitionTopic(), (int)this.m_key.partitionIndex());
        
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
    
    public boolean ethereal() {
        return this.isEthereal;
    }
}
