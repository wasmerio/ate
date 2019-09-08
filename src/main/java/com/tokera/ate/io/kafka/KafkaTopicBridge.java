package com.tokera.ate.io.kafka;

import com.tokera.ate.KafkaServer;
import com.tokera.ate.common.ApplicationConfigLoader;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.GenericPartitionKey;
import com.tokera.ate.dao.MessageBundle;
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
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.Response;
import java.io.IOException;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentSkipListSet;
import java.util.stream.Collectors;

/**
 * Represents the bridge of a particular Kafka topic
 */
public class KafkaTopicBridge implements IDataTopicBridge {
    protected AteDelegate d = AteDelegate.get();
    protected LoggerHook LOG = new LoggerHook(KafkaTopicBridge.class);

    private final String topic;
    private final DataPartitionType type;
    private final KafkaInbox inbox;
    private final KafkaOutbox outbox;
    private final Map<Integer, GenericPartitionBridge> partitionBridges;

    private final Random rand = new Random();
    private ConcurrentHashMap<MessageSyncDto, Object> syncs = new ConcurrentHashMap<>();
    private ConcurrentHashMap<Integer, Boolean> isLoaded = new ConcurrentHashMap<>();
    
    public KafkaTopicBridge(String topic, DataPartitionType topicType, KafkaInbox inbox, KafkaOutbox outbox)
    {
        this.topic = topic;
        this.type = topicType;
        this.inbox = inbox;
        this.outbox = outbox;
        this.partitionBridges = new HashMap<>();
    }

    public IDataPartitionBridge addKey(IPartitionKey key) {
        if (key.partitionTopic().equals(topic) ==false) {
            throw new WebApplicationException("Partition key does not match this topic.");
        }
        if (key.partitionIndex() >= KafkaTopicFactory.maxPartitionsPerTopic) {
            throw new WebApplicationException("Partition index can not exceed the maximum of " + KafkaTopicFactory.maxPartitionsPerTopic + " per topic.");
        }

        // Create the topic if it doesnt exist
        createTopicForPartition(key.partitionIndex());

        // Create the partition bridge if it does not exist
        GenericPartitionBridge ret;
        synchronized (this.partitionBridges) {
            ret = partitionBridges.computeIfAbsent(key.partitionIndex(), i -> new GenericPartitionBridge(this, key, new DataPartitionChain(key)));
        }
        inbox.reload();
        return ret;
    }

    private void createTopicForPartition(Integer forPartition) {
        // Make sure the topic is actually created
        KafkaTopicFactory.Response response = d.kafkaTopicFactory.create(this.topic, this.type);
        switch (response) {
            case AlreadyExists: {
                isLoaded.putIfAbsent(forPartition, Boolean.FALSE);
                break;
            }
            case WasCreated: {
                isLoaded.putIfAbsent(forPartition, Boolean.TRUE);
                break;
            }
            case Failed: {
                throw new WebApplicationException("Failed to create the new partitions.", Response.Status.INTERNAL_SERVER_ERROR);
            }
        }
    }

    public boolean removeKey(IPartitionKey key) {
        if (this.topic.equals(key.partitionTopic()) == false) {
            return false;
        }

        synchronized (this.partitionBridges) {
            boolean ret = partitionBridges.remove(key.partitionIndex()) != null;
            if (ret == false) return false;
            isLoaded.remove((Integer)key.partitionIndex());
            inbox.reload();
            return true;
        }
    }

    @Override
    public Set<IPartitionKey> keys() {
        synchronized (this.partitionBridges) {
            return this.partitionBridges.keySet().stream().map(i -> new GenericPartitionKey(this.topic, i, type)).collect(Collectors.toSet());
        }
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
        boolean hasCreated = false;
        boolean startedReload = false;

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
                if (waitTime.getTime() > 8000L) {
                    if (startedReload == false) {
                        this.inbox.reload();
                        startedReload = true;
                    }
                }
                if (waitTime.getTime() > 15000L) {
                    if (hasCreated == false) {
                        createTopicForPartition(key.partitionIndex());
                        this.inbox.reload();
                        hasCreated = true;
                    }
                }
                if (waitTime.getTime() > 25000L) {
                    StringBuilder sb  = new StringBuilder();
                    synchronized (this.partitionBridges) {
                        this.partitionBridges.keySet().forEach(i -> {
                            if (sb.length() > 0) sb.append(",");
                            sb.append(i);
                        });
                    }
                    throw new RuntimeException("Busy loading data partition [topic=" + this.topic + ", partitions=" + sb.toString() + "]");
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
        KafkaProducer<String, MessageBase> p = this.outbox.get();
        if (p != null) p.send(record);

        d.debugLogging.logKafkaSend(record, msg);
    }
   
    public @Nullable MessageDataDto getVersion(PUUID id, MessageMetaDto meta) {
        TopicPartition tp = new TopicPartition(id.partitionTopic(), id.partitionIndex());
        
        List<TopicPartition> tps = new LinkedList<>();
        tps.add(tp);
        
        KafkaConsumer<String, MessageBase> onceConsumer = d.kafkaConfig.newConsumer(KafkaConfigTools.TopicRole.Consumer, KafkaConfigTools.TopicType.Dao, KafkaServer.getKafkaBootstrap());
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

    @Override
    public void feed(Iterable<MessageBundle> msgs) {
        synchronized (this.partitionBridges)
        {
            // Now find the bridge and send the message to it
            for  (MessageBundle bundle : msgs) {
                int partition = bundle.partition;

                IDataPartitionBridge partitionBridge = MapTools.getOrNull(this.partitionBridges, partition);
                if (partitionBridge != null) {
                    // Now process the message itself
                    MessageMetaDto meta = new MessageMetaDto(
                            partition,
                            bundle.offset);

                    if (bundle.msg.msgType() == MessageType.MessageSync) {
                        processSync(new MessageSyncDto(bundle.msg), LOG);
                        return;
                    }
                    try {
                        boolean loaded = isLoaded.getOrDefault((Integer) partition, false);
                        partitionBridge.chain().rcv(bundle.msg, meta, loaded, LOG);
                    } catch (IOException | InvalidCipherTextException ex) {
                        LOG.warn(ex);
                    }
                }
            }
        }
    }

    @Override
    public void feedIdle(Iterable<Integer> partitions) {
        for (Integer p : partitions) {
            if (isLoaded.get(p) == false) {
                isLoaded.put(p, true);
            }
        }
    }
}
