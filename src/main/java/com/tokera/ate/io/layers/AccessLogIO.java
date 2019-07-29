package com.tokera.ate.io.layers;

import java.util.List;
import java.util.Set;
import java.util.UUID;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.io.api.IAteIO;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.core.RequestAccessLog;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.io.repo.DataTransaction;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;
import com.tokera.ate.io.repo.DataContainer;
import com.tokera.ate.io.repo.DataSubscriber;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.inject.spi.CDI;

/**
 * IO implementation that logs all reads and writes performed during a particular currentRights before forwarding the
 * currentRights onto downstream IO modules.
 * The primary use-case for this IO module is for cache-invalidation.
 */
final public class AccessLogIO implements IAteIO {

    private IAteIO next;
    private final RequestAccessLog logger;

    public AccessLogIO(IAteIO next) {
        this.next = next;
        this.logger = CDI.current().select(RequestAccessLog.class).get();
    }

    final protected <T extends BaseDao> void onRead(Class<T> clazz) {
        logger.recordRead(clazz);
    }

    final protected void onRead(UUID id, Class<?> clazz) {
        logger.recordRead(id, clazz);
    }

    final protected void onWrote(UUID id, Class<?> clazz) {
        logger.recordWrote(id, clazz);
    }

    @Override
    public @Nullable MessageDataHeaderDto readRootOfTrust(PUUID id) {
        return next.readRootOfTrust(id);
    }

    @Override
    public void warm(IPartitionKey partitionKey) { next.warm(partitionKey); }

    @Override
    public void warmAndWait(IPartitionKey partitionKey) { next.warmAndWait(partitionKey); }

    @Override
    public MessageSyncDto beginSync(IPartitionKey partitionKey, MessageSyncDto sync) {
        return next.beginSync(partitionKey, sync);
    }

    @Override
    public boolean finishSync(IPartitionKey partitionKey, MessageSyncDto sync) { return next.finishSync(partitionKey, sync); }

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
        BaseDao ret = next.readOrNull(id, shouldSave);
        if (ret != null) {
            this.onRead(id.id(), ret.getClass());
        }
        return ret;
    }

    @Override
    public BaseDao readOrThrow(PUUID id) {
        BaseDao ret = next.readOrThrow(id);
        if (ret != null) {
            this.onRead(id.id(), ret.getClass());
        }
        return ret;
    }

    @Override
    public @Nullable DataContainer readRawOrNull(PUUID id) { return next.readRawOrNull(id); }

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
        this.onRead(id.id(), clazz);
        return next.readHistory(id, clazz);
    }

    @Override
    public Set<BaseDao> readAll(IPartitionKey partitionKey) {
        return next.readAll(partitionKey);
    }

    @Override
    public <T extends BaseDao> Set<T> readAll(IPartitionKey partitionKey, Class<T> type) {
        Set<T> ret = next.readAll(partitionKey, type);
        this.onRead(type);
        return ret;
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
