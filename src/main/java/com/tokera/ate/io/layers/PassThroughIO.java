package com.tokera.ate.io.layers;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.io.api.IAteIO;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.DataTransaction;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;
import com.tokera.ate.io.repo.DataContainer;
import com.tokera.ate.io.repo.DataSubscriber;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.*;

/**
 * IO implementation that simple passes through IO commands to the next IO module with built in callbacks
 */
public class PassThroughIO implements IAteIO {
    protected final IAteIO next;

    public PassThroughIO(IAteIO next) {
        this.next = next;
    }

    @Override
    public @Nullable MessageDataHeaderDto readRootOfTrust(PUUID id) {
        return next.readRootOfTrust(id);
    }

    @Override
    public void warm(IPartitionKey partitionKey) {
        next.warm(partitionKey);
    }

    @Override
    public void warmAndWait(IPartitionKey partitionKey) { next.warmAndWait(partitionKey); }

    @Override
    public MessageSyncDto beginSync(IPartitionKey partitionKey, MessageSyncDto sync) {
        return next.beginSync(partitionKey, sync);
    }

    @Override
    public boolean finishSync(IPartitionKey partitionKey, MessageSyncDto sync) {
        return next.finishSync(partitionKey, sync);
    }

    @Override
    public DataSubscriber backend() {
        return next.backend();
    }

    @Override
    public @Nullable MessagePublicKeyDto publicKeyOrNull(IPartitionKey partitionKey, @Hash String hash) {
        return next.publicKeyOrNull(partitionKey, hash);
    }

    @Override
    public void send(DataTransaction transaction, boolean validate) {
        next.send(transaction, validate);
    }

    @Override
    public boolean exists(@Nullable PUUID _id) {
        @DaoId PUUID id = _id;
        if (id == null) return false;
        return next.exists(id);
    }

    @Override
    public boolean everExisted(@Nullable PUUID _id) {
        @DaoId PUUID id = _id;
        if (id == null) return false;
        return next.everExisted(id);
    }

    @Override
    public boolean immutable(PUUID id) {
        return next.immutable(id);
    }

    @Override
    public @Nullable BaseDao readOrNull(PUUID id, boolean shouldSave) {
        return next.readOrNull(id, shouldSave);
    }

    @Override
    public BaseDao readOrThrow(PUUID id) {
        return next.readOrThrow(id);
    }

    @Override
    public @Nullable DataContainer readRawOrNull(PUUID id) {
        return next.readRawOrNull(id);
    }

    @Override
    public @Nullable BaseDao readVersionOrNull(PUUID id, MessageMetaDto meta) {
        return next.readVersionOrNull(id, meta);
    }

    @Override
    public @Nullable MessageDataDto readVersionMsgOrNull(PUUID id, MessageMetaDto meta) {
        return next.readVersionMsgOrNull(id, meta);
    }

    @Override
    public <T extends BaseDao> Iterable<MessageMetaDto> readHistory(PUUID id, Class<T> clazz) {
        return next.readHistory(id, clazz);
    }

    @Override
    public List<BaseDao> readAll(IPartitionKey partitionKey) {
        return next.readAll(partitionKey);
    }

    @Override
    public List<BaseDao> readAllAccessible(IPartitionKey partitionKey) {
        return next.readAllAccessible(partitionKey);
    }

    @Override
    public <T extends BaseDao> List<T> readAll(IPartitionKey partitionKey, Class<T> type) {
        return next.readAll(partitionKey, type);
    }

    @Override
    public <T extends BaseDao> List<T> readAllAccessible(IPartitionKey partitionKey, Class<T> type) {
        return next.readAllAccessible(partitionKey, type);
    }

    @Override
    public <T extends BaseDao> List<DataContainer> readAllRaw(IPartitionKey partitionKey) {
        return next.readAllRaw(partitionKey);
    }

    @Override
    public <T extends BaseDao> List<DataContainer> readAllRaw(IPartitionKey partitionKey, Class<T> type) {
        return next.readAllRaw(partitionKey, type);
    }
}
