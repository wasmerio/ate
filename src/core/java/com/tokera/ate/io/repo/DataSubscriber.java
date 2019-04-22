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
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.enumerations.DataTopicType;
import com.tokera.ate.events.TopicSeedingEvent;
import org.checkerframework.checker.nullness.qual.NonNull;

import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;
import javax.enterprise.inject.spi.CDI;
import javax.ws.rs.WebApplicationException;

/**
 * Class used to build subscriptions to particular partitions and feed basic raw IO commands to it
 */
public class DataSubscriber {

    private AteDelegate d = AteDelegate.getUnsafe();
    private final LoggerHook LOG;
    private final Cache<String, @NonNull DataTopic> topicCache;
    private final ConcurrentHashMap<String, RamTopicPartition> ramPartitions;
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
                .removalListener((RemovalNotification<String, DataTopic> notification) -> {
                    DataTopic t = notification.getValue();
                    if (t != null) t.stop();
                })
                .build();
        this.ramPartitions = new ConcurrentHashMap<>();
    }
    
    private void seedTopic(DataTopic kt)
    {   
        DataTopicChain chain = kt.getChain();
        d.eventTopicSeeding.fire(new TopicSeedingEvent(kt, chain));
    }
    
    public DataTopic getTopic(String topicName) {
        return getTopic(topicName, true, DataTopicType.Dao);
    }
    
    public DataTopicChain getChain(String topicName) {
        DataTopic topic = getTopic(topicName);
        return topic.getChain();
    }

    private IDataTopicBridge createBridge(DataTopicChain chain, DataTopicType type) {
        if (this.mode == Mode.Ram) {
            RamTopicPartition p = this.ramPartitions.computeIfAbsent(chain.getTopicName(), RamTopicPartition::new);
            return new RamTopicBridge(chain, type, p);
        } else {
            return d.kafkaBridgeBuilder.build(chain, type);
        }
    }

    private DataTopic createTopic(String topicName, DataTopicType type) {
        DataTopicChain chain = new DataTopicChain(
                topicName,
                d.daoParents.getAllowedParentsSimple(),
                d.daoParents.getAllowedParentFreeSimple());
        IDataTopicBridge bridge = createBridge(chain, type);

        DataTopic newTopic = new DataTopic(chain, bridge, type, d.daoParents);
        newTopic.start();
        seedTopic(newTopic);
        return newTopic;
    }
    
    public DataTopic getTopic(String topicName, boolean shouldWait, DataTopicType type) {
        DataTopic ret = this.topicCache.getIfPresent(topicName);
        if (ret != null) {
            if (shouldWait == true) {
                ret.waitTillLoaded();
            }
            return ret;
        }

        try
        {
            ret = this.topicCache.get(topicName, () ->
                {
                    synchronized(this)
                    {
                        LOG.info("loading-topic: " + topicName);
                        d.encryptor.touch(); // required as the kafka topic needs an instance reference
                        return createTopic(topicName, type);
                    }
                });
        } catch (ExecutionException ex) {
            throw new WebApplicationException(ex);
        }
        
        if (shouldWait == true) {
            ret.waitTillLoaded();
        }

        return ret;
    }
    
    public DataTopicChain getChain(String topicName, boolean shouldWait) {
        DataTopic topic = getTopic(topicName, shouldWait, DataTopicType.Dao);
        return topic.getChain();
    }
    
    public void touch() {
    }
}
