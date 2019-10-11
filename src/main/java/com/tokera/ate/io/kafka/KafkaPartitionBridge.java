package com.tokera.ate.io.kafka;

import com.tokera.ate.KafkaServer;
import com.tokera.ate.dao.MessageBundle;
import com.tokera.ate.dao.TopicAndPartition;
import com.tokera.ate.dao.kafka.MessageSerializer;
import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.dao.msg.MessageData;
import com.tokera.ate.dao.msg.MessageType;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageBaseDto;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageMetaDto;
import com.tokera.ate.dto.msg.MessageSyncDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.DataPartitionChain;
import com.tokera.ate.io.repo.IDataPartitionBridge;
import com.tokera.ate.providers.PartitionKeySerializer;
import org.apache.commons.lang3.time.StopWatch;
import org.apache.kafka.clients.consumer.ConsumerRecord;
import org.apache.kafka.clients.consumer.ConsumerRecords;
import org.apache.kafka.clients.consumer.KafkaConsumer;
import org.apache.kafka.clients.producer.KafkaProducer;
import org.apache.kafka.clients.producer.ProducerRecord;
import org.apache.kafka.common.TopicPartition;
import org.bouncycastle.crypto.InvalidCipherTextException;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.Response;
import java.io.IOException;
import java.time.Duration;
import java.util.LinkedList;
import java.util.List;
import java.util.UUID;

public class KafkaPartitionBridge implements IDataPartitionBridge {
    public final AteDelegate d;
    public final IPartitionKey key;
    public final DataPartitionChain chain;
    private volatile MessageSyncDto loadSync = null;

    public KafkaPartitionBridge(AteDelegate d, IPartitionKey key, DataPartitionChain chain) {
        this.d = d;
        this.key = key;
        this.chain = chain;
    }

    @Override
    public void send(MessageBaseDto msg)
    {
        // Send the message do Kafka
        ProducerRecord<String, MessageBase> record = new ProducerRecord<>(key.partitionTopic(), key.partitionIndex(), MessageSerializer.getKey(msg), msg.createBaseFlatBuffer());

        // Send the record to Kafka
        KafkaProducer<String, MessageBase> p = d.kafkaOutbox.get();
        if (p != null) p.send(record);

        d.debugLogging.logKafkaSend(record, msg);
    }

    @Override
    public @Nullable MessageDataDto getVersion(UUID id, long offset) {
        TopicPartition tp = new TopicPartition(key.partitionTopic(), key.partitionIndex());

        List<TopicPartition> tps = new LinkedList<>();
        tps.add(tp);

        KafkaConsumer<String, MessageBase> onceConsumer = d.kafkaConfig.newConsumer(KafkaConfigTools.TopicRole.Consumer, KafkaConfigTools.TopicType.Dao, KafkaServer.getKafkaBootstrap());
        onceConsumer.assign(tps);
        onceConsumer.seek(tp, offset);

        final ConsumerRecords<String, MessageBase> consumerRecords = onceConsumer.poll(Duration.ofMillis(5000));
        if (consumerRecords.isEmpty()) return null;

        for (ConsumerRecord<String, MessageBase> msg : consumerRecords) {
            if (msg.partition() == this.partitionKey().partitionIndex() &&
                    msg.offset() == offset)
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

    public void sendLoadSync() {
        MessageSyncDto sync = d.partitionSyncManager.startSync();
        this.send(sync);
        this.loadSync = sync;

        d.debugLogging.logBeginLoad(this.key);
    }

    @Override
    public void waitTillLoaded() {
        boolean sentSync = false;
        boolean hasCreated = false;
        boolean startedReload = false;

        if (loadSync != null) {
            StopWatch waitTime = new StopWatch();
            waitTime.start();
            while (d.partitionSyncManager.hasFinishSync(this.loadSync) == false) {
                if (waitTime.getTime() > 5000L) {
                    if (sentSync == false) {
                        sendLoadSync();
                        sentSync = true;
                    }
                }
                if (waitTime.getTime() > 8000L) {
                    if (startedReload == false) {
                        d.kafkaInbox.addPartition(new TopicAndPartition(key));
                        startedReload = true;
                    }
                }
                if (waitTime.getTime() > 15000L) {
                    if (hasCreated == false) {
                        createTopic();
                        d.kafkaInbox.addPartition(new TopicAndPartition(key));
                        hasCreated = true;
                    }
                }
                if (waitTime.getTime() > 25000L) {
                    throw new RuntimeException("Busy loading data partition [" + PartitionKeySerializer.toString(key) + "]");
                }
                try {
                    Thread.sleep(50);
                } catch (InterruptedException ex) {
                    return;
                }
            }

            d.debugLogging.logFinishLoad(this.key);
            this.loadSync = null;
        }
    }

    public void createTopic()
    {
        // Make sure the topic is actually created
        KafkaTopicFactory.Response response = AteDelegate.get().kafkaTopicFactory.create(key.partitionTopic(), key.partitionType());
        switch (response) {
            case AlreadyExists: {
                break;
            }
            case WasCreated: {
                AteDelegate.get().genericLogger.info("partition [" + this.key + "]: loaded-created");
                break;
            }
            case Failed: {
                throw new WebApplicationException("Failed to create the new partitions.", Response.Status.INTERNAL_SERVER_ERROR);
            }
        }
    }

    @Override
    public IPartitionKey partitionKey() {
        return this.key;
    }

    @Override
    public DataPartitionChain chain() {
        return this.chain;
    }

    @Override
    public boolean hasLoaded() {
        return loadSync == null;
    }
}
