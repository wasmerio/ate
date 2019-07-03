package com.tokera.ate.io.layers;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.io.api.IAteIO;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;
import com.tokera.ate.io.repo.DataContainer;
import com.tokera.ate.io.repo.DataRepository;
import com.tokera.ate.io.repo.DataSubscriber;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.Collection;
import java.util.List;
import java.util.Set;
import java.util.UUID;

/**
 * IO implementation that simple passes through IO commands to the next IO module with built in callbacks
 */
final public class BackendIO implements IAteIO {

    private final DataRepository next;
    private final DataSubscriber backend;

    public BackendIO(DataRepository next, DataSubscriber backend) {
        this.next = next;
        this.backend = backend;
    }

    @Override
    public boolean merge(BaseDao t) {
        return next.merge(t);
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
        return next.mergeAsync(t);
    }

    @Override
    public boolean mergeWithoutValidation(BaseDao t) {
        return next.mergeWithoutValidation(t);
    }

    @Override
    public boolean mergeAsyncWithoutValidation(BaseDao t) {
        return next.mergeAsyncWithoutValidation(t);
    }
    
    @Override
    public void mergeLater(BaseDao t) {
        next.mergeLater(t);
    }

    @Override
    public void mergeLaterWithoutValidation(BaseDao t) {
        next.mergeLaterWithoutValidation(t);
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
        return next.remove(t);
    }
    
    @Override
    public void removeLater(BaseDao t) {
        next.removeLater(t);
    }
    
    @Override
    public boolean remove(PUUID id, Class<?> type) {
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
    public void warm(IPartitionKey partitionKey) {
        next.warm(partitionKey);
    }

    @Override
    public void sync(IPartitionKey partitionKey) {
        next.sync(partitionKey);
    }

    @Override
    public boolean sync(IPartitionKey partitionKey, MessageSyncDto sync) {
        return next.sync(partitionKey, sync);
    }

    @Override
    public @Nullable MessagePublicKeyDto publicKeyOrNull(IPartitionKey partitionKey, @Hash String hash) {
        return next.publicKeyOrNull(partitionKey, hash);
    }
    
    @Override
    public boolean exists(@Nullable PUUID id) {
        return next.exists(id);
    }
    
    @Override
    public boolean everExisted(@Nullable PUUID id) {
        return next.everExisted(id);
    }
    
    @Override
    public boolean immutable(PUUID id) {
        return next.immutable(id);
    }

    @Override
    public @Nullable BaseDao getOrNull(PUUID id, boolean shouldSave) {
        return next.getOrNull(id, shouldSave);
    }

    @Override
    public BaseDao getOrThrow(PUUID id) {
        return next.getOrThrow(id);
    }

    @Override
    public @Nullable DataContainer getRawOrNull(PUUID id) {
        return next.getRawOrNull(id);
    }
    
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
        return next.getHistory(id, clazz);
    }

    @Override
    public Set<BaseDao> getAll(IPartitionKey partitionKey) {
        return next.getAll(partitionKey);
    }

    @Override
    public <T extends BaseDao> Set<T> getAll(IPartitionKey partitionKey, Class<T> type) {
        return next.getAll(partitionKey, type);
    }

    @Override
    public <T extends BaseDao> Set<T> getAll(Collection<IPartitionKey> keys, Class<T> type) {
        return next.getAll(keys, type);
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
    public <T extends BaseDao> List<T> getMany(IPartitionKey partitionKey, Iterable<@DaoId UUID> ids, Class<T> type) {
        return next.getMany(partitionKey, ids, type);
    }

    @Override
    public DataSubscriber backend() {
        return this.backend;
    }
}
