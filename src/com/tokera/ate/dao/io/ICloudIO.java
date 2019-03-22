package com.tokera.ate.dao.io;

import com.tokera.server.api.dao.BaseDao;
import com.tokera.server.api.dto.EffectivePermissions;
import com.tokera.server.api.dto.msg.*;
import com.tokera.server.api.repositories.DataContainer;
import com.tokera.server.api.units.DaoId;
import com.tokera.server.api.units.Hash;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.Collection;
import java.util.List;
import java.util.Set;
import java.util.UUID;

/**
 * Interface used for generic input output operations on data entities
 */
public interface ICloudIO {
    
    boolean merge(BaseDao entity);

    boolean merge(MessagePublicKeyDto publicKey);

    boolean merge(MessageEncryptTextDto encryptText);

    void mergeLater(BaseDao entity);

    boolean remove(BaseDao entity);
    
    boolean remove(@DaoId UUID id, Class<?> type);
    
    void removeLater(BaseDao entity);

    void cache(BaseDao entity);

    boolean exists(@Nullable @DaoId UUID id);
    
    boolean ethereal();
    
    boolean everExisted(@Nullable @DaoId UUID id);
    
    boolean immutable(@DaoId UUID id);
    
    EffectivePermissions perms(@DaoId UUID id, @Nullable @DaoId UUID parentId, boolean usePostMerged);

    @Nullable BaseDao getOrNull(@DaoId UUID id);

    @Nullable DataContainer getRawOrNull(@DaoId UUID id);
    
    <T extends BaseDao> Iterable<MessageMetaDto> getHistory(@DaoId UUID id, Class<T> clazz);
    
    @Nullable BaseDao getVersionOrNull(@DaoId UUID id, MessageMetaDto meta);
    
    @Nullable MessageDataDto getVersionMsgOrNull(@DaoId UUID id, MessageMetaDto meta);

    <T extends BaseDao> Set<T> getAll();
    
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
}
