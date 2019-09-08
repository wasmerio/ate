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
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Set;
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

    private int poll()
    {
        // Process all the records using this polling timeout
        int foundRecords = 0;
        int emptyCount = 0;
        while (true)
        {
            KafkaConsumer<String, MessageBase> c = get();
            Multimap<String, Integer> idleTopics = HashMultimap.create();
            c.assignment().stream()
                .map(a -> new TopicAndPartition(a.topic(), a.partition()))
                .forEach(a -> {
                    idleTopics.put(a.partitionTopic(), (Integer)a.partitionIndex());
                });

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

            String curTopic = null;
            ArrayList<MessageBundle> msgs = new ArrayList<>();

            for (ConsumerRecord<String, MessageBase> record : consumerRecords) {
                idleTopics.remove(record.topic(), (Integer)record.partition());

                if (curTopic != null && curTopic.equals(record.topic()) == false) {
                    d.dataRepository.feed(curTopic, msgs);
                    msgs.clear();
                }

                curTopic = record.topic();
                msgs.add(new MessageBundle(record.partition(), record.offset(), record.value()));
            }
            if (curTopic != null) {
                d.dataRepository.feed(curTopic, msgs);
            }

            for (String topic : idleTopics.keySet()) {
                d.dataRepository.feedIdle(topic, idleTopics.get(topic));
            }
        }

        return foundRecords;
    }
}
