package com.tokera.ate.io.repo;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dto.msg.MessageBaseDto;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageMetaDto;
import com.tokera.ate.dto.msg.MessageSyncDto;
import com.tokera.ate.io.api.IPartitionKey;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.UUID;

public class GenericPartitionBridge implements IDataPartitionBridge {
    private final IDataTopicBridge bridge;
    private final IPartitionKey key;
    private final DataPartitionChain chain;

    public GenericPartitionBridge(IDataTopicBridge bridge, IPartitionKey key, DataPartitionChain chain) {
        this.bridge = bridge;
        this.key = key;
        this.chain = chain;
    }

    @Override
    public void send(MessageBaseDto msg) {
        bridge.send(key, msg);
    }

    @Override
    public void waitTillLoaded() {
        bridge.waitTillLoaded(key);
    }

    @Override
    public boolean sync() {
        return bridge.sync(key);
    }

    @Override
    public MessageSyncDto startSync() {
        return bridge.startSync(key);
    }

    @Override
    public boolean finishSync(MessageSyncDto sync) {
        return bridge.finishSync(key, sync);
    }

    @Override
    public boolean finishSync(MessageSyncDto sync, int timeout) {
        return bridge.finishSync(key, sync, timeout);
    }

    @Override
    public boolean hasFinishSync(MessageSyncDto sync) {
        return bridge.hasFinishSync(key, sync);
    }

    @Override
    public @Nullable MessageDataDto getVersion(UUID id, MessageMetaDto meta) {
        return bridge.getVersion(PUUID.from(key, id), meta);
    }

    @Override
    public IDataTopicBridge topicBridge() {
        return this.bridge;
    }

    @Override
    public IPartitionKey partitionKey() {
        return this.key;
    }

    @Override
    public DataPartitionChain chain() {
        return this.chain;
    }
}
