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
import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.events.PartitionSeedingEvent;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.ram.RamPartitionBridge;
import com.tokera.ate.io.ram.RamTopicPartition;
import org.checkerframework.checker.nullness.qual.NonNull;

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
    private final Cache<GenericPartitionKey, @NonNull DataPartition> topicCache;
    private final ConcurrentHashMap<GenericPartitionKey, RamTopicPartition> ramPartitions;
    private final Mode mode;

    public enum Mode {
        Ram,
        Kafka
    }

    public DataSubscriber(Mode mode) {
        this.mode = mode;
        this.LOG = CDI.current().select(LoggerHook.class).get();
        this.topicCache = CacheBuilder.newBuilder()
                .maximumSize(500)
                .expireAfterAccess(1, TimeUnit.MINUTES)
                .removalListener((RemovalNotification<GenericPartitionKey, DataPartition> notification) -> {
                    DataPartition t = notification.getValue();
                    if (t != null) t.stop();
                })
                .build();
        this.ramPartitions = new ConcurrentHashMap<>();
    }
    
    private void seedTopic(DataPartition kt)
    {   
        DataPartitionChain chain = kt.getChain();
        d.eventTopicSeeding.fire(new PartitionSeedingEvent(kt, chain));
    }
    
    public DataPartition getPartition(IPartitionKey partition) {
        return getPartition(partition, true, DataPartitionType.Dao);
    }
    
    public DataPartitionChain getChain(IPartitionKey partitionKey) {
        DataPartition partition = getPartition(partitionKey);
        return partition.getChain();
    }

    private IDataPartitionBridge createBridge(IPartitionKey key, DataPartitionChain chain, DataPartitionType type) {
        GenericPartitionKey keyWrap = new GenericPartitionKey(key);
        if (this.mode == Mode.Ram) {
            RamTopicPartition p = this.ramPartitions.computeIfAbsent(keyWrap, RamTopicPartition::new);
            return new RamPartitionBridge(chain, type, p);
        } else {
            return d.kafkaBridgeBuilder.build(keyWrap, chain, type);
        }
    }

    private DataPartition createPartition(IPartitionKey key, DataPartitionType type) {
        DataPartitionChain chain = new DataPartitionChain(key);
        IDataPartitionBridge bridge = createBridge(key, chain, type);

        DataPartition newTopic = new DataPartition(key, chain, bridge, type, d.daoParents);
        newTopic.start();
        seedTopic(newTopic);
        return newTopic;
    }
    
    public DataPartition getPartition(IPartitionKey partition, boolean shouldWait, DataPartitionType type) {
        GenericPartitionKey keyWrap = new GenericPartitionKey(partition);
        DataPartition ret = this.topicCache.getIfPresent(keyWrap);
        if (ret != null) {
            if (shouldWait == true) {
                ret.waitTillLoaded();
            }
            return ret;
        }

        try
        {
            ret = this.topicCache.get(keyWrap, () ->
                {
                    synchronized(this)
                    {
                        LOG.info("loading-partition: " + keyWrap.partitionTopic() + ":" + keyWrap.partitionIndex());
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
}
