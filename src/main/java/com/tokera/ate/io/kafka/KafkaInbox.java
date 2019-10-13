package com.tokera.ate.io.kafka;

import com.tokera.ate.KafkaServer;
import com.tokera.ate.dao.MessageBundle;
import com.tokera.ate.dao.TopicAndPartition;
import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.io.core.DataPartitionDaemon;
import com.tokera.ate.io.repo.DataSubscriber;
import com.tokera.ate.scopes.Startup;
import org.apache.kafka.clients.consumer.ConsumerRecord;
import org.apache.kafka.clients.consumer.ConsumerRecords;
import org.apache.kafka.clients.consumer.KafkaConsumer;
import org.apache.kafka.common.TopicPartition;

import javax.enterprise.context.ApplicationScoped;
import java.time.Duration;
import java.util.*;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;
import java.util.stream.Collectors;

@Startup
@ApplicationScoped
public class KafkaInbox extends DataPartitionDaemon {

    private int pollTimeout = 10;
    protected AtomicInteger targetInit = new AtomicInteger(0);
    private AtomicInteger initLevel = new AtomicInteger(-1);
    private AtomicReference<KafkaConsumer<String, MessageBase>> consumer = new AtomicReference<>();

    @Override
    public void addPartition(TopicAndPartition what) {
        targetInit.incrementAndGet();
        super.addPartition(what);
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
        Integer curInitLevel = initLevel.get();
        Integer newInitLevel = targetInit.get();
        if (curInitLevel != newInitLevel) {
            if (initLevel.compareAndSet(curInitLevel, newInitLevel)) {
                load();
            }
        }
    }

    private void load()
    {
        // Build a list of all the partitions that need to be assigned to
        Set<TopicAndPartition> keys = listPartitions();
        Set<TopicPartition> kafkaPartitions = keys.stream()
                .map(k -> new TopicPartition(k.partitionTopic(), k.partitionIndex()))
                .collect(Collectors.toSet());

        // Get the list of the stuff already assigned
        KafkaConsumer<String, MessageBase> c = get();
        Set<TopicAndPartition> existing = c.assignment().stream()
                .map(a -> new TopicAndPartition(a.topic(), a.partition()))
                .collect(Collectors.toSet());

        // Check for fast exit
        if (existing.size() == keys.size() &&
                existing.stream().filter(a -> keys.contains(a)).count() == keys.size()) {
            return;
        }

        // Update the consumer
        c.assign(kafkaPartitions);

        // Determine what partitions need to be reset and put them back to offset zero
        List<TopicPartition> restart = new ArrayList<>();
        for (TopicAndPartition p : keys) {
            if (existing.contains(p)) continue;
            restart.add(new TopicPartition(p.partitionTopic(), p.partitionIndex()));
        }
        if (restart.size() > 0) {
            c.seekToBeginning(restart);
        }

        // Trigger anyone thats waiting
        this.notifyAll();
    }

    @Override
    protected void work() throws InterruptedException {
        touchLoad();
        poll();
    }

    // Called when the main thread is idle
    private void idle()
    {

    }

    private void poll()
    {
        // Build a list of all the topics and partitions we are interested in
        final KafkaConsumer<String, MessageBase> c = get();
        final Set<TopicAndPartition> idlePartitions = c.assignment().stream()
                .map(a -> new TopicAndPartition(a.topic(), a.partition()))
                .collect(Collectors.toSet());
        if (idlePartitions.size() <= 0) {
            try {
                Thread.sleep(pollTimeout);
            } catch (InterruptedException e) {
            }
            return;
        }

        // Wait for data to arrive from Kafka
        final ConsumerRecords<String, MessageBase> consumerRecords =
                c.poll(Duration.ofMillis(pollTimeout));
        if (consumerRecords.isEmpty()) {
            idle();
            return;
        }

        // Group all the messages into topics
        final Map<TopicAndPartition, ArrayList<MessageBundle>> msgs = new HashMap<>();
        for (ConsumerRecord<String, MessageBase> record : consumerRecords)
        {
            // If we have a record for the topic and partition then its obviously not idle anymore
            TopicAndPartition key = new TopicAndPartition(record.topic(), record.partition());
            idlePartitions.remove(key);

            // Add it to the bundle
            msgs.computeIfAbsent(key, k -> new ArrayList<>())
                .add(new MessageBundle(record.key(), record.partition(), record.offset(), record.value()));
        }

        DataSubscriber subscriber = AteDelegate.get().storageFactory.get().backend();

        // Now in a parallel engine that increases throughput we stream all the data into the repositories
        msgs.entrySet()
            .parallelStream()
            .forEach(e -> subscriber.feed(e.getKey(), e.getValue(), false));
    }
}
