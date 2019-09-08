/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.io.repo;

import com.google.common.cache.Cache;
import com.google.common.cache.CacheBuilder;
import com.google.common.cache.RemovalNotification;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.MessageBundle;
import com.tokera.ate.dao.TopicAndPartition;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.events.KeysDiscoverEvent;
import com.tokera.ate.events.PartitionSeedingEvent;
import com.tokera.ate.io.api.IPartitionKey;
import org.checkerframework.checker.nullness.qual.NonNull;

import java.lang.ref.WeakReference;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.stream.Collectors;
import javax.enterprise.inject.spi.CDI;

/**
 * Class used to build subscriptions to particular partitions and feed basic raw IO commands to it
 */
public class DataSubscriber {

    private AteDelegate d = AteDelegate.get();
    private final LoggerHook LOG;
    private final Cache<TopicAndPartition, @NonNull DataPartition> partitions;
    private final ConcurrentHashMap<String, @NonNull WeakReference<IDataTopicBridge>> bridges;
    private final Mode mode;

    public enum Mode {
        Ram,
        Kafka
    }
    public DataSubscriber(Mode mode) {
        this.mode = mode;
        this.LOG = CDI.current().select(LoggerHook.class).get();
        this.bridges = new ConcurrentHashMap<>();
        this.partitions = CacheBuilder.newBuilder()
                .maximumSize(d.bootstrapConfig.getSubscriberMaxPartitions())
                .expireAfterAccess(d.bootstrapConfig.getSubscriberPartitionTimeout(), TimeUnit.MILLISECONDS)
                .build();
    }

    private void seedTopic(DataPartition kt)
    {   
        DataPartitionChain chain = kt.getChain();
        d.eventTopicSeeding.fire(new PartitionSeedingEvent(kt, chain));

        KeysDiscoverEvent discovery = new KeysDiscoverEvent(kt.partitionKey());
        d.eventKeysDiscovery.fire(discovery);

        for (MessagePublicKeyDto key : discovery.getKeys()) {
            chain.addTrustKey(key, this.LOG);
        }
    }
    
    public DataPartition getPartition(IPartitionKey partition) {
        return getPartition(partition, true);
    }
    
    public DataPartitionChain getChain(IPartitionKey partitionKey) {
        DataPartition partition = getPartition(partitionKey);
        return partition.getChain();
    }

    private IDataTopicBridge getOrCreateTopicBridge(String topic, DataPartitionType type) {
        WeakReference<IDataTopicBridge> weak = this.bridges
                .computeIfAbsent(topic, k -> new WeakReference<>(null));
        IDataTopicBridge ret = weak.get();
        if (ret != null) return ret;
        ret = createTopicBridge(topic, type);
        this.bridges.put(topic, new WeakReference<>(ret));
        return ret;
    }

    private IDataTopicBridge createTopicBridge(String topic, DataPartitionType type) {
        IDataTopicBridge ret;
        if (this.mode == Mode.Ram) {
            ret = d.ramBridgeBuilder.build(topic, type);
        } else {
            ret = d.kafkaBridgeBuilder.build(topic, type);
        }
        return ret;
    }

    private DataPartition createPartition(IPartitionKey key) {
        IDataTopicBridge topicBridge = getOrCreateTopicBridge(key.partitionTopic(), key.partitionType());
        IDataPartitionBridge partitionBridge = topicBridge.createPartition(key);
        DataPartition newTopic = new DataPartition(key, partitionBridge, d.daoParents);
        seedTopic(newTopic);
        return newTopic;
    }

    public DataPartition getPartition(IPartitionKey key, boolean shouldWait) {
        TopicAndPartition keyWrap = new TopicAndPartition(key);
        DataPartition ret = this.partitions.getIfPresent(keyWrap);
        if (ret != null) {
            if (shouldWait == true) {
                ret.waitTillLoaded();
            }
            return ret;
        }

        try
        {
            ret = this.partitions.get(keyWrap, () ->
                {
                    d.debugLogging.logLoadingPartition(key);
                    d.encryptor.touch(); // required as the kafka partition needs an instance reference
                    return createPartition(key);
                });
        } catch (ExecutionException ex) {
            throw new RuntimeException(ex);
        }
        
        if (shouldWait == true) {
            ret.waitTillLoaded();
        }

        return ret;
    }
    
    public void touch() {
    }

    public void destroyAll() {
        this.partitions.invalidateAll();
        synchronized (bridges) {
            bridges.clear();
        }
    }

    public Set<TopicAndPartition> keys() {
        return this.partitions.asMap()
                .keySet()
                .stream()
                .collect(Collectors.toSet());
    }

    public void feed(TopicAndPartition where, Iterable<MessageBundle> msgs) {
        DataPartition ret = this.partitions.getIfPresent(where);
        if (ret != null) ret.feed(msgs);
    }

    public void feedIdle(TopicAndPartition where) {
        DataPartition ret = this.partitions.getIfPresent(where);
        if (ret != null) ret.idle();
    }
}
