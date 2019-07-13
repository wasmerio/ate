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
import com.tokera.ate.dao.GenericPartitionKey;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.events.KeysDiscoverEvent;
import com.tokera.ate.events.NewAccessRightsEvent;
import com.tokera.ate.events.PartitionSeedingEvent;
import com.tokera.ate.io.api.IPartitionKey;
import org.checkerframework.checker.nullness.qual.NonNull;

import java.lang.ref.WeakReference;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;
import javax.enterprise.inject.spi.CDI;

/**
 * Class used to build subscriptions to particular partitions and feed basic raw IO commands to it
 */
public class DataSubscriber {

    private AteDelegate d = AteDelegate.get();
    private final LoggerHook LOG;
    private final Cache<GenericPartitionKey, @NonNull DataPartition> partitionCache;
    private final Map<String, @NonNull WeakReference<IDataTopicBridge>> topicCache;
    private final Mode mode;

    public enum Mode {
        Ram,
        Kafka
    }

    public DataSubscriber(Mode mode) {
        this.mode = mode;
        this.LOG = CDI.current().select(LoggerHook.class).get();
        this.partitionCache = CacheBuilder.newBuilder()
                .maximumSize(500)
                .expireAfterAccess(1, TimeUnit.MINUTES)
                .removalListener((RemovalNotification<GenericPartitionKey, DataPartition> notification) -> {
                    DataPartition t = notification.getValue();
                    if (t != null) t.getBridge().topicBridge().removeKey(t.partitionKey());
                })
                .build();
        this.topicCache = new ConcurrentHashMap<>();
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
        synchronized (topicCache) {
            WeakReference<IDataTopicBridge> weak = this.topicCache
                    .computeIfAbsent(topic, k -> new WeakReference<>(null));
            IDataTopicBridge ret = weak.get();
            if (ret != null) return ret;
            ret = createTopicBridge(topic, type);
            this.topicCache.put(topic, new WeakReference<>(ret));
            return ret;
        }
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

    private DataPartition createPartition(IPartitionKey key, DataPartitionType type) {
        IDataTopicBridge topicBridge = getOrCreateTopicBridge(key.partitionTopic(), type);
        IDataPartitionBridge partitionBridge = topicBridge.addKey(key);
        DataPartition newTopic = new DataPartition(key, partitionBridge, type, d.daoParents);
        seedTopic(newTopic);
        return newTopic;
    }

    public DataPartition getPartition(IPartitionKey partition, boolean shouldWait) {
        return getPartition(partition, shouldWait, DataPartitionType.Dao);
    }
    
    public DataPartition getPartition(IPartitionKey partition, boolean shouldWait, DataPartitionType type) {
        GenericPartitionKey keyWrap = new GenericPartitionKey(partition);
        DataPartition ret = this.partitionCache.getIfPresent(keyWrap);
        if (ret != null) {
            if (shouldWait == true) {
                ret.waitTillLoaded();
            }
            return ret;
        }

        try
        {
            ret = this.partitionCache.get(keyWrap, () ->
                {
                    synchronized(this)
                    {
                        d.debugLogging.logLoadingPartition(keyWrap, this.LOG);
                        d.encryptor.touch(); // required as the kafka partition needs an instance reference
                        return createPartition(keyWrap, type);
                    }
                });
        } catch (ExecutionException ex) {
            throw new RuntimeException(ex);
        }
        
        if (shouldWait == true) {
            ret.waitTillLoaded();
        }

        return ret;
    }
    
    public DataPartitionChain getChain(IPartitionKey key, boolean shouldWait) {
        DataPartition partition = getPartition(key, shouldWait, DataPartitionType.Dao);
        return partition.getChain();
    }
    
    public void touch() {
    }

    public void destroyAll() {
        partitionCache.invalidateAll();
        synchronized (topicCache) {
            topicCache.clear();
        }
    }
}
