package com.tokera.ate.io.layers;

import java.util.Collection;
import java.util.List;
import java.util.Set;
import java.util.UUID;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.io.api.IAteIO;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.core.RequestAccessLog;
import com.tokera.ate.dto.msg.*;
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
    public boolean merge(BaseDao t) {
        boolean ret = next.merge(t);
        if (ret == false) return false;
        this.onWrote(t.getId(), t.getClass());
        return true;
    }

    @Override
    public boolean merge(IPartitionKey partitionKey, MessagePublicKeyDto publicKey) {
        return this.next.merge(partitionKey, publicKey);
    }

    @Override
    public boolean merge(IPartitionKey partitionKey, MessageSecurityCastleDto castle) {
        return this.next.merge(partitionKey, castle);
    }

    @Override
    public boolean mergeAsync(BaseDao t) {
        boolean ret = next.mergeAsync(t);
        if (ret == false) return false;
        this.onWrote(t.getId(), t.getClass());
        return true;
    }

    @Override
    public boolean mergeWithoutValidation(BaseDao t) {
        boolean ret = next.mergeWithoutValidation(t);
        if (ret == false) return false;
        this.onWrote(t.getId(), t.getClass());
        return true;
    }

    @Override
    public boolean mergeAsyncWithoutValidation(BaseDao t) {
        boolean ret = next.mergeAsyncWithoutValidation(t);
        if (ret == false) return false;
        this.onWrote(t.getId(), t.getClass());
        return true;
    }

    @Override
    public void mergeLater(BaseDao t) {
        next.mergeLater(t);
        this.onWrote(t.getId(), t.getClass());
    }

    @Override
    public void mergeLaterWithoutValidation(BaseDao t) {
        next.mergeLaterWithoutValidation(t);
        this.onWrote(t.getId(), t.getClass());
    }

    @Override
    public void mergeDeferred() {
        next.mergeDeferred();
    }

    @Override
    public void clearDeferred() {
        next.clearDeferred();
    }

    @Override
    public void clearCache(PUUID id) {
        next.clearCache(id);
    }

    @Override
    public boolean remove(BaseDao t) {
        this.onWrote(t.getId(), t.getClass());
        return next.remove(t);
    }

    @Override
    public void removeLater(BaseDao t) {
        this.onWrote(t.getId(), t.getClass());
        next.removeLater(t);
    }

    @Override
    public boolean remove(PUUID id, Class<?> type) {
        this.onWrote(id.id(), type);
        return next.remove(id, type);
    }

    @Override
    public void cache(BaseDao entity) {
        next.cache(entity);
    }

    @Override
    public void decache(BaseDao entity) {
        next.decache(entity);
    }

    @Override
    public @Nullable MessageDataHeaderDto getRootOfTrust(PUUID id) {
        return next.getRootOfTrust(id);
    }

    @Override
    public void warm(IPartitionKey partitionKey) { next.warm(partitionKey); }

    @Override
    public void sync(IPartitionKey partitionKey) { next.sync(partitionKey); }

    @Override
    public boolean sync(IPartitionKey partitionKey, MessageSyncDto sync) { return next.sync(partitionKey, sync); }

    @Override
    public DataSubscriber backend() {
        return next.backend();
    }

    @Override
    public @Nullable MessagePublicKeyDto publicKeyOrNull(IPartitionKey partitionKey, @Hash String hash) {
        return next.publicKeyOrNull(partitionKey, hash);
    }

    @Override
    public boolean exists(@Nullable PUUID _id) {
        @DaoId PUUID id = _id;
        if (id == null) return false;
        return next.exists(id);
    }

    @Override
    public boolean ethereal(IPartitionKey partitionKey) {
        return next.ethereal(partitionKey);
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
    public @Nullable BaseDao getOrNull(PUUID id) {
        BaseDao ret = next.getOrNull(id);
        if (ret != null) {
            this.onRead(id.id(), ret.getClass());
        }
        return ret;
    }

    @Override
    public BaseDao getOrThrow(PUUID id) {
        BaseDao ret = next.getOrThrow(id);
        if (ret != null) {
            this.onRead(id.id(), ret.getClass());
        }
        return ret;
    }

    @Override
    public @Nullable DataContainer getRawOrNull(PUUID id) { return next.getRawOrNull(id); }

    @Override
    public @Nullable BaseDao getVersionOrNull(PUUID id, MessageMetaDto meta) {
        return next.getVersionOrNull(id, meta);
    }

    @Override
    public @Nullable MessageDataDto getVersionMsgOrNull(PUUID id, MessageMetaDto meta) {
        return next.getVersionMsgOrNull(id, meta);
    }

    @Override
    public <T extends BaseDao> Iterable<MessageMetaDto> getHistory(PUUID id, Class<T> clazz) {
        this.onRead(id.id(), clazz);
        return next.getHistory(id, clazz);
    }

    @Override
    public Set<BaseDao> getAll(IPartitionKey partitionKey) {
        return next.getAll(partitionKey);
    }

    @Override
    public <T extends BaseDao> Set<T> getAll(IPartitionKey partitionKey, Class<T> type) {
        Set<T> ret = next.getAll(partitionKey, type);
        this.onRead(type);
        return ret;
    }

    @Override
    public <T extends BaseDao> List<DataContainer> getAllRaw(IPartitionKey partitionKey) {
        return next.getAllRaw(partitionKey);
    }

    @Override
    public <T extends BaseDao> List<DataContainer> getAllRaw(IPartitionKey partitionKey, Class<T> type) {
        return next.getAllRaw(partitionKey, type);
    }

    @Override
    public <T extends BaseDao> List<T> getMany(IPartitionKey partitionKey, Iterable<@DaoId  UUID> ids, Class<T> type) {
        List<T> ret = next.getMany(partitionKey, ids, type);
        for (T entity : ret) {
            this.onRead(entity.getId(), type);
        }
        return ret;
    }
}
