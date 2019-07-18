package com.tokera.ate.io.layers;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.base.BaseDaoInternal;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.io.api.*;
import com.tokera.ate.qualifiers.BackendStorageSystem;
import com.tokera.ate.qualifiers.FrontendStorageSystem;
import com.tokera.ate.units.*;
import com.tokera.ate.io.repo.DataContainer;
import com.tokera.ate.io.repo.DataSubscriber;
import org.checkerframework.checker.nullness.qual.EnsuresNonNullIf;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import java.util.*;

/**
 * Generic IO class used to access the IO subsystem without forcing it to be loaded before its initialized. Also
 * includes a bunch of built in helper classes that are best not placed in the interface itself
 */
@FrontendStorageSystem
@ApplicationScoped
public class HeadIO implements IAteIO
{
    protected AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    @BackendStorageSystem
    protected IAteIO back;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    @BackendStorageSystem
    protected IPartitionResolver backPartitionResolver;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    @BackendStorageSystem
    protected IPartitionKeyMapper backPartitionKeyMapper;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    @BackendStorageSystem
    protected ISecurityCastleFactory backSecurityCastleFactory;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    @BackendStorageSystem
    protected ITokenParser backTokenParser;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;

    public HeadIO() {
    }

    @Override
    public boolean merge(BaseDao t) {
        return back.merge(t);
    }

    @Override
    public boolean merge(IPartitionKey partitionKey, MessagePublicKeyDto publicKey) {
        return this.back.merge(partitionKey, publicKey);
    }

    @Override
    public boolean merge(IPartitionKey partitionKey, MessageSecurityCastleDto castle) {
        return this.back.merge(partitionKey, castle);
    }

    @Override
    public boolean mergeAsync(BaseDao t) {
        return back.mergeAsync(t);
    }

    @Override
    public boolean mergeWithoutValidation(BaseDao t) {
        return back.mergeWithoutValidation(t);
    }

    @Override
    public boolean mergeAsyncWithoutValidation(BaseDao t) {
        return back.mergeAsyncWithoutValidation(t);
    }

    @Override
    public void mergeLater(BaseDao t) {
        back.mergeLater(t);
    }

    @Override
    public void mergeLaterWithoutValidation(BaseDao t) {
        back.mergeLaterWithoutValidation(t);
    }

    @Override
    public void mergeDeferred() {
        back.mergeDeferred();
    }

    public void mergeDeferredAndSync() {
        this.mergeDeferred();
        this.sync();
    }

    @Override
    public void clearDeferred() {
        back.clearDeferred();
    }

    public void clearCache(@DaoId UUID id) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        back.clearCache(PUUID.from(partitionKey, id));
    }

    @Override
    public void clearCache(PUUID id) {
        back.clearCache(id);
    }

    @Override
    public boolean remove(BaseDao t) {
        return back.remove(t);
    }

    @Override
    public void removeLater(BaseDao t) {
        back.removeLater(t);
    }

    public boolean remove(@DaoId UUID id, Class<?> type) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return back.remove(PUUID.from(partitionKey, id), type);
    }

    @Override
    public boolean remove(PUUID id, Class<?> type) {
        return back.remove(id, type);
    }

    @Override
    public void cache(BaseDao entity) {
        back.cache(entity);
    }

    @Override
    public void decache(BaseDao entity) {
        back.decache(entity);
    }

    public void warm()
    {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        back.warm(partitionKey);
    }

    public void warmAndWait()
    {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        back.warmAndWait(partitionKey);
    }

    @Override
    public void warm(IPartitionKey partitionKey) { back.warm(partitionKey); }

    @Override
    public void warmAndWait(IPartitionKey partitionKey) { back.warmAndWait(partitionKey); }

    public void sync()
    {
        for (IPartitionKey partitionKey : d.dataStagingManager.getTouchedPartitions()) {
            back.sync(partitionKey);
        }
    }

    @Override
    public void sync(IPartitionKey partitionKey) { back.sync(partitionKey); }

    @Override
    public boolean sync(IPartitionKey partitionKey, MessageSyncDto sync) { return back.sync(partitionKey, sync); }

    @Override
    public DataSubscriber backend() {
        return back.backend();
    }

    public IPartitionResolver partitionResolver() {
        return this.backPartitionResolver;
    }

    public IPartitionKeyMapper partitionKeyMapper() { return this.backPartitionKeyMapper; }

    public ISecurityCastleFactory securityCastleFactory() {
        return this.backSecurityCastleFactory;
    }
    
    public ITokenParser tokenParser() { return this.backTokenParser; }

    public @Nullable MessagePublicKeyDto publicKeyOrNull(@Hash String hash) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScopeOrNull();
        if (partitionKey != null) {
            @Nullable MessagePublicKeyDto ret = back.publicKeyOrNull(partitionKey, hash);
            if (ret != null) return ret;
        }
        for (IPartitionKey otherKey : d.requestContext.getOtherPartitionKeys()) {
            @Nullable MessagePublicKeyDto ret = back.publicKeyOrNull(otherKey, hash);
            if (ret != null) return ret;
        }
        return null;
    }

    @Override
    public @Nullable MessagePublicKeyDto publicKeyOrNull(IPartitionKey partitionKey, @Nullable @Hash String _hash) {
        @Hash String hash = _hash;
        if (hash == null) return null;
        return back.publicKeyOrNull(partitionKey, hash);
    }

    public boolean exists(@Nullable @DaoId UUID _id) {
        @DaoId UUID id = _id;
        if (id == null) return false;
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return back.exists(PUUID.from(partitionKey, id));
    }

    @EnsuresNonNullIf(expression="#1", result=true)
    public boolean exists(IPartitionKey partitionKey, @DaoId UUID id) {
        return back.exists(PUUID.from(partitionKey, id));
    }

    @Override
    @EnsuresNonNullIf(expression="#1", result=true)
    public boolean exists(@Nullable PUUID id) {
        if (id == null) return false;
        return back.exists(id);
    }

    public boolean everExisted(@Nullable @DaoId UUID _id) {
        @DaoId UUID id = _id;
        if (id == null) return false;
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return back.everExisted(PUUID.from(partitionKey, id));
    }

    @Override
    public boolean everExisted(@Nullable PUUID id){
        if (id == null) return false;
        return back.everExisted(id);
    }

    public boolean immutable(@DaoId UUID id) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return back.immutable(PUUID.from(partitionKey, id));
    }

    @Override
    public boolean immutable(PUUID id) {
        return back.immutable(id);
    }

    public @Nullable MessageDataHeaderDto getRootOfTrust(@DaoId UUID id) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return back.getRootOfTrust(PUUID.from(partitionKey, id));
    }

    @Override
    public @Nullable MessageDataHeaderDto getRootOfTrust(PUUID id) {
        return back.getRootOfTrust(id);
    }

    public @Nullable BaseDao getOrNull(@DaoId UUID id) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return back.getOrNull(PUUID.from(partitionKey, id), true);
    }

    public @Nullable BaseDao getOrNull(PUUID id) {
        return back.getOrNull(id, true);
    }

    public @Nullable BaseDao getOrNull(PUUID id, boolean shouldSave) {
        return back.getOrNull(id, shouldSave);
    }

    @Override
    public BaseDao getOrThrow(PUUID id) {
        return back.getOrThrow(id);
    }

    public <T extends BaseDao> T get(@DaoId UUID id, Class<T> type) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return this.get(PUUID.from(partitionKey, id), type);
    }

    @SuppressWarnings({"unchecked"})
    public <T extends BaseDao> T get(PUUID id, Class<T> type) {
        try {
            BaseDao ret = back.getOrThrow(id);
            if (ret == null) {
                throw new RuntimeException(type.getSimpleName() + " not found (id=" + id.print() + ")");
            }
            if (ret.getClass() != type) {
                throw new RuntimeException(type.getSimpleName() + " of the wrong type (id=" + id.print() + ", actual=" + ret.getClass().getSimpleName() + ", expected=" + type.getSimpleName() + ")");
            }
            BaseDaoInternal.assertStillMutable(ret);
            return (T)ret;
        } catch (ClassCastException ex) {
            throw new RuntimeException(type.getSimpleName() + " of the wrong type (id=" + id.print() + ")", ex);
        }
    }

    protected BaseDao get(@DaoId UUID id) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return this.get(PUUID.from(partitionKey, id));
    }

    protected BaseDao get(PUUID id) {
        BaseDao ret = back.getOrThrow(id);
        if (ret == null) {
            throw new RuntimeException("Object data (id=" + id.print() + ") not found");
        }
        return ret;
    }

    public DataContainer getRaw(@DaoId UUID id)
    {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return this.getRaw(PUUID.from(partitionKey, id));
    }

    public DataContainer getRaw(PUUID id)
    {
        DataContainer ret = back.getRawOrNull(id);
        if (ret == null) {
            throw new RuntimeException("Object data (id=" + id.print() + ") not found");
        }
        return ret;
    }

    public @Nullable DataContainer getRawOrNull(@DaoId UUID id)
    {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return back.getRawOrNull(PUUID.from(partitionKey, id));
    }

    @Override
    public @Nullable DataContainer getRawOrNull(PUUID id)
    {
        return back.getRawOrNull(id);
    }

    public <T extends BaseDao> Iterable<MessageMetaDto> getHistory(@DaoId UUID id, Class<T> clazz) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return back.getHistory(PUUID.from(partitionKey, id), clazz);
    }

    @Override
    public <T extends BaseDao> Iterable<MessageMetaDto> getHistory(PUUID id, Class<T> clazz) {
        return back.getHistory(id, clazz);
    }

    public @Nullable BaseDao getVersionOrNull(@DaoId UUID id, MessageMetaDto meta) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return back.getVersionOrNull(PUUID.from(partitionKey, id), meta);
    }

    @Override
    public @Nullable BaseDao getVersionOrNull(PUUID id, MessageMetaDto meta) {
        return back.getVersionOrNull(id, meta);
    }

    public @Nullable MessageDataDto getVersionMsgOrNull(@DaoId UUID id, MessageMetaDto meta) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return back.getVersionMsgOrNull(PUUID.from(partitionKey, id), meta);
    }

    @Override
    public @Nullable MessageDataDto getVersionMsgOrNull(PUUID id, MessageMetaDto meta) {
        return back.getVersionMsgOrNull(id, meta);
    }

    public BaseDao getVersion(@DaoId UUID id, MessageMetaDto meta) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return this.getVersion(PUUID.from(partitionKey, id), meta);
    }

    public BaseDao getVersion(PUUID id, MessageMetaDto meta) {
        BaseDao ret = back.getVersionOrNull(id, meta);
        if (ret == null) {
            throw new RuntimeException("Object version data (id=" + id.print() + ") not found");
        }
        return ret;
    }

    public MessageDataDto getVersionMsg(@DaoId UUID id, MessageMetaDto meta) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return this.getVersionMsg(PUUID.from(partitionKey, id), meta);
    }

    public MessageDataDto getVersionMsg(PUUID id, MessageMetaDto meta) {
        MessageDataDto ret = back.getVersionMsgOrNull(id, meta);
        if (ret == null) {
            throw new RuntimeException("Object version message (id=" + id.print() + ") not found");
        }
        return ret;
    }

    public Set<BaseDao> getAll() {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return back.getAll(partitionKey);
    }

    @Override
    public Set<BaseDao> getAll(IPartitionKey partitionKey) {
        return back.getAll(partitionKey);
    }

    public <T extends BaseDao> Set<T> getAll(Class<T> type) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return back.getAll(partitionKey, type);
    }

    @Override
    public <T extends BaseDao> Set<T> getAll(IPartitionKey partitionKey, Class<T> type) {
        return back.getAll(partitionKey, type);
    }

    @Override
    public <T extends BaseDao> Set<T> getAll(Collection<IPartitionKey> keys, Class<T> type) {
        return back.getAll(keys, type);
    }

    public <T extends BaseDao> List<DataContainer> getAllRaw()
    {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return back.getAllRaw(partitionKey);
    }

    @Override
    public <T extends BaseDao> List<DataContainer> getAllRaw(IPartitionKey partitionKey) { return back.getAllRaw(partitionKey); }

    public <T extends BaseDao> List<DataContainer> getAllRaw(Class<T> type)
    {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return back.getAllRaw(partitionKey, type);
    }

    @Override
    public <T extends BaseDao> List<DataContainer> getAllRaw(IPartitionKey partitionKey, Class<T> type) { return back.getAllRaw(partitionKey, type); }

    public <T extends BaseDao> List<T> getMany(Iterable<@DaoId UUID> ids, Class<T> type) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        return back.getMany(partitionKey, ids, type);
    }

    @Override
    public <T extends BaseDao> List<T> getMany(IPartitionKey partitionKey, Iterable<@DaoId UUID> ids, Class<T> type) {
        return back.getMany(partitionKey, ids, type);
    }

    public <T extends BaseDao> List<T> getManyAcrossPartitions(Iterable<PUUID> ids, Class<T> type) {
        ArrayList<T> ret = new ArrayList<>();
        for (PUUID id : ids) {
            ret.add(this.get(id, type));
        }
        return ret;
    }
}
