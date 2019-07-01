package com.tokera.ate.io.ram;

import com.tokera.ate.dao.GenericPartitionKey;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.kafka.KafkaTopicBridge;
import com.tokera.ate.io.repo.DataPartitionChain;
import com.tokera.ate.io.repo.IDataPartitionBridge;
import com.tokera.ate.io.repo.IDataTopicBridge;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.ws.rs.WebApplicationException;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.stream.Collectors;

/**
 * Represents a bridge of a particular partition with an in memory RAM copy of the data
 */
public class RamTopicBridge implements IDataTopicBridge {
    private final String topic;
    private final DataPartitionType type;
    private final ConcurrentHashMap<Integer, RamPartitionBridge> ramBridges;

    private static final ConcurrentHashMap<IPartitionKey, RamTopicPartition> allRamPartitions = new ConcurrentHashMap<>();

    public RamTopicBridge(String topic, DataPartitionType type) {
        this.topic = topic;
        this.type = type;
        this.ramBridges = new ConcurrentHashMap<>();
    }

    private RamPartitionBridge getOrCreateBridge(IPartitionKey key) {
        if (key.partitionTopic().equals(topic) ==false) {
            throw new WebApplicationException("Partition key does not match this topic.");
        }
        if (key.partitionIndex() >= KafkaTopicBridge.maxPartitionsPerTopic) {
            throw new WebApplicationException("Partition index can not exceed the maximum of " + KafkaTopicBridge.maxPartitionsPerTopic + " per topic.");
        }

        GenericPartitionKey wrapKey = new GenericPartitionKey(key);
        return ramBridges.computeIfAbsent(key.partitionIndex(), a -> {
            RamTopicPartition data = allRamPartitions.computeIfAbsent(key, i -> new RamTopicPartition(wrapKey));
            DataPartitionChain chain = new DataPartitionChain(key);
            return new RamPartitionBridge(this, chain, type, data);
        });
    }

    @Override
    public void send(IPartitionKey key, MessageBaseDto msg) {
        getOrCreateBridge(key).send(msg);
    }

    @Override
    public void waitTillLoaded(IPartitionKey key) {
        getOrCreateBridge(key).waitTillLoaded();
    }

    @Override
    public IDataPartitionBridge addKey(IPartitionKey key) {
        RamPartitionBridge bridge = getOrCreateBridge(key);
        return bridge;
    }

    @Override
    public boolean removeKey(IPartitionKey key) {
        if (key.partitionTopic().equals(topic) ==false) {
            throw new WebApplicationException("Partition key does not match this topic.");
        }
        return ramBridges.remove(key.partitionIndex()) != null;
    }

    @Override
    public Set<IPartitionKey> keys() {
        return ramBridges.values().stream().map(b -> b.partitionKey()).collect(Collectors.toSet());
    }

    @Override
    public boolean sync(IPartitionKey key) {
        return getOrCreateBridge(key).sync();
    }

    @Override
    public MessageSyncDto startSync(IPartitionKey key) {
        return getOrCreateBridge(key).startSync();
    }

    @Override
    public boolean finishSync(IPartitionKey key, MessageSyncDto sync) {
        return getOrCreateBridge(key).finishSync(sync);
    }

    @Override
    public boolean finishSync(IPartitionKey key, MessageSyncDto sync, int timeout) {
        return getOrCreateBridge(key).finishSync(sync, timeout);
    }

    @Override
    public boolean hasFinishSync(IPartitionKey key, MessageSyncDto sync) {
        return getOrCreateBridge(key).hasFinishSync(sync);
    }

    @Override
    public @Nullable MessageDataDto getVersion(PUUID id, MessageMetaDto meta) {
        return getOrCreateBridge(id.partition()).getVersion(id.id(), meta);
    }
}