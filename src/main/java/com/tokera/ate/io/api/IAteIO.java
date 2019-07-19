package com.tokera.ate.io.api;

import java.util.Collection;
import java.util.List;
import java.util.UUID;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.io.repo.DataContainer;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;
import com.tokera.ate.io.repo.DataSubscriber;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.opensaml.xml.signature.P;

import java.util.Set;

/**
 * Interface used for generic input output operations on data entities
 */
public interface IAteIO {

    boolean merge(IPartitionKey partitionKey, MessagePublicKeyDto publicKey);

    boolean merge(IPartitionKey partitionKey, MessageSecurityCastleDto castle);

    boolean merge(BaseDao entity);

    boolean mergeAsync(BaseDao entity);

    boolean mergeWithoutSync(BaseDao entity);

    boolean mergeWithoutValidation(BaseDao entity);

    boolean mergeAsyncWithoutValidation(BaseDao entity);

    void mergeLater(BaseDao entity);

    void mergeLaterWithoutValidation(BaseDao entity);

    boolean remove(BaseDao entity);
    
    boolean remove(PUUID id, Class<?> type);
    
    void removeLater(BaseDao entity);

    void cache(BaseDao entity);

    void decache(BaseDao entity);

    boolean exists(@Nullable PUUID id);
    
    boolean everExisted(@Nullable PUUID id);
    
    boolean immutable(PUUID id);

    @Nullable MessageDataHeaderDto getRootOfTrust(PUUID id);

    @Nullable BaseDao getOrNull(PUUID id, boolean shouldSave);

    BaseDao getOrThrow(PUUID id);

    @Nullable DataContainer getRawOrNull(PUUID id);
    
    <T extends BaseDao> Iterable<MessageMetaDto> getHistory(PUUID id, Class<T> clazz);
    
    @Nullable BaseDao getVersionOrNull(PUUID id, MessageMetaDto meta);
    
    @Nullable MessageDataDto getVersionMsgOrNull(PUUID id, MessageMetaDto meta);

    Set<BaseDao> getAll(IPartitionKey partitionKey);
    
    <T extends BaseDao> Set<T> getAll(IPartitionKey partitionKey, Class<T> type);

    <T extends BaseDao> Set<T> getAll(Collection<IPartitionKey> keys, Class<T> type);

    <T extends BaseDao> List<DataContainer> getAllRaw(IPartitionKey partitionKey);

    <T extends BaseDao> List<DataContainer> getAllRaw(IPartitionKey partitionKey, Class<T> type);
    
    <T extends BaseDao> List<T> getMany(IPartitionKey partitionKey, Iterable<@DaoId UUID> ids, Class<T> type);

    @Nullable MessagePublicKeyDto publicKeyOrNull(IPartitionKey partitionKey, @Hash String hash);
    
    void mergeDeferred();
    
    void clearDeferred();
    
    void clearCache(PUUID id);

    void warm(IPartitionKey partitionKey);

    void warmAndWait(IPartitionKey partitionKey);

    void sync(IPartitionKey partitionKey);

    boolean sync(IPartitionKey partitionKey, MessageSyncDto sync);

    DataSubscriber backend();
}
