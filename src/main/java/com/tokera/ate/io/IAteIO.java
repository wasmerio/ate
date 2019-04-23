package com.tokera.ate.io;

import java.util.Collection;
import java.util.List;
import java.util.UUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.io.repo.DataContainer;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;
import com.tokera.ate.io.repo.DataSubscriber;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.Set;

/**
 * Interface used for generic input output operations on data entities
 */
public interface IAteIO {

    boolean merge(MessagePublicKeyDto publicKey);

    boolean merge(MessageEncryptTextDto encryptText);

    boolean merge(BaseDao entity);

    boolean mergeAsync(BaseDao entity);

    boolean mergeWithoutValidation(BaseDao entity);

    boolean mergeAsyncWithoutValidation(BaseDao entity);

    void mergeLater(BaseDao entity);

    void mergeLaterWithoutValidation(BaseDao entity);

    boolean remove(BaseDao entity);
    
    boolean remove(@DaoId UUID id, Class<?> type);
    
    void removeLater(BaseDao entity);

    void cache(BaseDao entity);

    void decache(BaseDao entity);

    boolean exists(@Nullable @DaoId UUID id);
    
    boolean ethereal();
    
    boolean everExisted(@Nullable @DaoId UUID id);
    
    boolean immutable(@DaoId UUID id);

    @Nullable MessageDataHeaderDto getRootOfTrust(UUID id);

    @Nullable BaseDao getOrNull(@DaoId UUID id);

    @Nullable DataContainer getRawOrNull(@DaoId UUID id);
    
    <T extends BaseDao> Iterable<MessageMetaDto> getHistory(@DaoId UUID id, Class<T> clazz);
    
    @Nullable BaseDao getVersionOrNull(@DaoId UUID id, MessageMetaDto meta);
    
    @Nullable MessageDataDto getVersionMsgOrNull(@DaoId UUID id, MessageMetaDto meta);

    Set<BaseDao> getAll();
    
    <T extends BaseDao> Set<T> getAll(Class<T> type);

    <T extends BaseDao> List<DataContainer> getAllRaw();

    <T extends BaseDao> List<DataContainer> getAllRaw(Class<T> type);
    
    <T extends BaseDao> List<T> getMany(Collection<@DaoId UUID> ids, Class<T> type);

    @Nullable MessagePublicKeyDto publicKeyOrNull(@Hash String hash);
    
    void mergeDeferred();
    
    void clearDeferred();
    
    void clearCache(@DaoId UUID id);
    
    void warm();

    void sync();

    boolean sync(MessageSyncDto sync);

    DataSubscriber backend();
}
