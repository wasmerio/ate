package com.tokera.ate.io.kafka;

import com.google.common.collect.HashMultimap;
import com.google.common.collect.Multimap;
import com.tokera.ate.KafkaServer;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.GenericPartitionKey;
import com.tokera.ate.dao.MessageBundle;
import com.tokera.ate.dao.TopicAndPartition;
import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.dao.msg.MessageType;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageMetaDto;
import com.tokera.ate.dto.msg.MessageSyncDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.IDataPartitionBridge;
import com.tokera.ate.scopes.Startup;
import org.apache.kafka.clients.consumer.ConsumerRecord;
import org.apache.kafka.clients.consumer.ConsumerRecords;
import org.apache.kafka.clients.consumer.KafkaConsumer;
import org.apache.kafka.common.TopicPartition;
import org.bouncycastle.crypto.InvalidCipherTextException;

import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import java.io.IOException;
import java.util.*;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;
import java.util.stream.Collectors;

@Startup
@ApplicationScoped
public class KafkaInbox implements Runnable {
    private AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;

    private Thread thread;
    private int pollTimeout = 10;
    private AtomicInteger isInit = new AtomicInteger(0);
    private AtomicInteger initLevel = new AtomicInteger(-1);
    private AtomicBoolean isRunning = new AtomicBoolean(false);

    private AtomicReference<KafkaConsumer<String, MessageBase>> consumer = new AtomicReference<>();

    public void reload() {
        isInit.incrementAndGet();
        if (isRunning.compareAndSet(false, true)) {
            this.thread = new Thread(this);
            this.thread.setDaemon(true);
            this.thread.start();
        }
    }

    private KafkaConsumer<String, MessageBase> get() {
        for (;;) {
            KafkaConsumer<String, MessageBase> ret = this.consumer.get();
            if (ret != null) return ret;

            synchronized (this) {
                ret = this.consumer.get();
                if (ret != null) return ret;

                ret = d.kafkaConfig.newConsumer(KafkaConfigTools.TopicRole.Consumer, KafkaConfigTools.TopicType.Dao, KafkaServer.getKafkaBootstrap());
                if (this.consumer.compareAndSet(null, ret) == true) {
                    return ret;
                } else {
                    ret.close();
                }
            }
        }
    }

    private void touchLoad() {
        Integer curInitLevel = isInit.get();
        Integer newInitLevel = initLevel.get();
        if (curInitLevel != newInitLevel) {
            if (initLevel.compareAndSet(curInitLevel, newInitLevel)) {
                load();
            }
        }
    }

    private void load() {
        Set<IPartitionKey> keys = d.dataRepository.keys();
        List<TopicPartition> partitions = keys.stream()
                .map(k -> new TopicPartition(k.partitionTopic(), k.partitionIndex()))
                .collect(Collectors.toList());

        KafkaConsumer<String, MessageBase> c = get();

        Set<TopicPartition> existing = c.assignment().stream().collect(Collectors.toSet());
        c.assign(partitions);

        List<TopicPartition> restart = new ArrayList<>();
        for (TopicPartition p : partitions) {
            if (existing.contains(p)) continue;
            restart.add(p);
        }

        c.seekToBeginning(restart);
    }

    @Override
    public void run() {
        try {
            for (; ; ) {
                try {
                    touchLoad();
                    poll();
                } catch (Throwable ex) {
                    LOG.warn(ex);

                    try {
                        Thread.sleep(5);
                    } catch (InterruptedException e) {
                        LOG.warn(ex);
                    }
                }
            }
        } finally {
            this.isRunning.set(false);
        }
    }

    private void poll()
    {
        // Process all the records using this polling timeout
        while (true)
        {
            // Build a list of all the topics and partitions we are interested in
            final KafkaConsumer<String, MessageBase> c = get();
            final Map<String, ArrayList<Integer>> idleTopics = new HashMap<>();
            c.assignment().stream()
                    .map(a -> new TopicAndPartition(a.topic(), a.partition()))
                    .forEach(a -> {
                        idleTopics.computeIfAbsent(a.partitionTopic(), k -> new ArrayList<>())
                                .add(a.partitionIndex());
                    });

            // Wait for data to arrive from Kafka
            final ConsumerRecords<String, MessageBase> consumerRecords =
                    c.poll(pollTimeout);

            // Group all the messages into topics
            final Map<String, ArrayList<MessageBundle>> msgs = new HashMap<>();
            for (ConsumerRecord<String, MessageBase> record : consumerRecords)
            {
                // If we have a record for the topic and partition then its obviously not idle anymore
                idleTopics.computeIfPresent(record.topic(), (a, b) -> {
                    b.remove(record.partition());
                    return b.size() > 0 ? b : null;
                });

                msgs.computeIfAbsent(record.topic(), k -> new ArrayList<>())
                    .add(new MessageBundle(record.partition(), record.offset(), record.value()));
            }

            // Now in a parallel engine that increases throughput we stream all the data into the repositories
            msgs.entrySet()
                .parallelStream()
                .forEach(e -> d.dataRepository.feed(e.getKey(), e.getValue()));

            // Finally we let any topics that didnt receive anything that they are now idle and thus can consider
            // themselves at this exact point in time to be as update-to-date as possible
            idleTopics.entrySet()
                .parallelStream()
                .filter(a -> a.getValue().size() > 0)
                .forEach(e -> d.dataRepository.feedIdle(e.getKey(), e.getValue()));
        }
    }
}
