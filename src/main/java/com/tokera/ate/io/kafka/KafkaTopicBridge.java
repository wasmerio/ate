package com.tokera.ate.io.kafka;

import com.tokera.ate.KafkaServer;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.MapTools;
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
import com.tokera.ate.io.repo.IDataPartitionBridge;
import com.tokera.ate.io.repo.IDataTopicBridge;
import org.apache.commons.lang3.time.StopWatch;
import org.apache.kafka.clients.consumer.ConsumerRecord;
import org.apache.kafka.clients.consumer.ConsumerRecords;
import org.apache.kafka.clients.consumer.KafkaConsumer;
import org.apache.kafka.clients.producer.KafkaProducer;
import org.apache.kafka.clients.producer.ProducerRecord;
import org.apache.kafka.common.TopicPartition;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.Response;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Represents the bridge of a particular Kafka topic
 */
public class KafkaTopicBridge implements IDataTopicBridge {
    protected AteDelegate d = AteDelegate.get();
    public final LoggerHook LOG = new LoggerHook(KafkaTopicBridge.class);

    public final String topic;
    public final DataPartitionType type;
    public final KafkaInbox inbox;
    public final KafkaOutbox outbox;

    private final Random rand = new Random();
    private ConcurrentHashMap<MessageSyncDto, Object> syncs = new ConcurrentHashMap<>();
    
    public KafkaTopicBridge(String topic, DataPartitionType topicType, KafkaInbox inbox, KafkaOutbox outbox)
    {
        this.topic = topic;
        this.type = topicType;
        this.inbox = inbox;
        this.outbox = outbox;
    }

    public IDataPartitionBridge createPartition(IPartitionKey key) {
        if (key.partitionTopic().equals(topic) ==false) {
            throw new WebApplicationException("Partition key does not match this topic.");
        }
        if (key.partitionIndex() >= KafkaTopicFactory.maxPartitionsPerTopic) {
            throw new WebApplicationException("Partition index can not exceed the maximum of " + KafkaTopicFactory.maxPartitionsPerTopic + " per topic.");
        }

        // Create the partition bridge if it does not exist
        KafkaPartitionBridge ret = new KafkaPartitionBridge(this, key, new DataPartitionChain(key));

        // Create the topic if it doesnt exist
        ret.createTopic();
        inbox.reload();
        return ret;
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
        return finishSync(key, sync, 60000);
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

    public void processSync(MessageSyncDto sync)
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
    
    public void send(IPartitionKey key, MessageBaseDto msg)
    {
        // Send the message do Kafka
        ProducerRecord<String, MessageBase> record = new ProducerRecord<>(key.partitionTopic(), key.partitionIndex(), MessageSerializer.getKey(msg), msg.createBaseFlatBuffer());
        
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
}
