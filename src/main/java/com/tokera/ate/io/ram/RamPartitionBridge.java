package com.tokera.ate.io.ram;

import com.tokera.ate.dao.MessageBundle;
import com.tokera.ate.dao.TopicAndPartition;
import com.tokera.ate.dao.kafka.MessageSerializer;
import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.DataPartitionChain;
import com.tokera.ate.io.repo.DataSubscriber;
import com.tokera.ate.io.repo.IDataPartitionBridge;

import java.util.*;

import org.bouncycastle.crypto.InvalidCipherTextException;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.io.IOException;

/**
 * Represents a bridge of a particular partition with an in memory RAM copy of the data
 */
public class RamPartitionBridge implements IDataPartitionBridge {

    private final AteDelegate d = AteDelegate.get();
    private final DataPartitionChain chain;
    private final DataPartitionType type;
    private final DataSubscriber subscriber;
    private final TopicAndPartition where;

    public RamPartitionBridge(DataPartitionChain chain, DataPartitionType type) {
        this.chain = chain;
        this.type = type;
        this.where = new TopicAndPartition(chain.partitionKey().partitionTopic(), chain.partitionKey().partitionIndex());
        this.subscriber = d.storageFactory.get().backend();
    }

    @Override
    public void send(MessageBaseDto msg) {
        MessageBase flat = msg.createBaseFlatBuffer();

        String key = MessageSerializer.getKey(msg);
        MessageBundle bundle = d.ramDataRepository.write(where, key, flat);
        this.subscriber.feed(this.where, Collections.singletonList(bundle), true);
        this.subscriber.idle(this.where);
    }

    @Override
    public void deleteMany(Collection<String> keys) {
        d.ramDataRepository.deleteMany(where, keys);
    }

    @Override
    public void waitTillLoaded() {
    }

    @Override
    public @Nullable MessageDataMetaDto getVersion(UUID id, long offset) {
        return d.ramDataRepository.getVersion(where, offset);
    }

    @Override
    public IPartitionKey partitionKey() {
        return this.chain.partitionKey();
    }

    @Override
    public DataPartitionChain chain() {
        return this.chain;
    }

    @Override
    public boolean hasLoaded() { return true; }
}