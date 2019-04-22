package com.tokera.ate.io;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.qualifiers.BackendStorageSystem;
import com.tokera.ate.qualifiers.FrontendStorageSystem;
import com.tokera.ate.units.*;
import com.tokera.ate.io.repo.DataContainer;
import com.tokera.ate.io.repo.DataSubscriber;
import org.checkerframework.checker.nullness.qual.EnsuresNonNullIf;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.Response;
import java.util.*;

/**
 * Generic IO class used to access the IO subsystem without forcing it to be loaded before its initialized. Also
 * includes a bunch of built in helper classes that are best not placed in the interface itself
 */
@FrontendStorageSystem
@ApplicationScoped
public class HeadIO implements IAteIO
{
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    @BackendStorageSystem
    protected IAteIO back;
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
    public boolean merge(MessagePublicKeyDto t) {
        return back.merge(t);
    }

    @Override
    public boolean merge(MessageEncryptTextDto t) {
        return back.merge(t);
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

    @Override
    public void clearDeferred() {
        back.clearDeferred();
    }

    @Override
    public void clearCache(@DaoId UUID id) {
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

    @Override
    public boolean remove(@DaoId UUID id, Class<?> type) {
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

    @Override
    public void warm() {
        back.warm();
    }

    @Override
    public void sync() { back.sync(); }

    @Override
    public boolean sync(MessageSyncDto sync) { return back.sync(sync); }

    @Override
    public DataSubscriber backend() {
        return back.backend();
    }

    @Override
    public @Nullable MessagePublicKeyDto publicKeyOrNull(@Hash String hash) {
        return back.publicKeyOrNull( hash);
    }

    @Override
    @EnsuresNonNullIf(expression="#1", result=true)
    public boolean exists(@Nullable @DaoId UUID id) {
        if (id == null) return false;
        return back.exists(id);
    }

    @Override
    public boolean ethereal() {
        return back.ethereal();
    }

    @Override
    public boolean everExisted(@Nullable @DaoId UUID id){
        if (id == null) return false;
        return back.everExisted(id);
    }

    @Override
    public boolean immutable(@DaoId UUID id) {
        return back.immutable(id);
    }

    @Override
    public @Nullable MessageDataHeaderDto getRootOfTrust(UUID id) {
        return back.getRootOfTrust(id);
    }

    @Override
    public @Nullable BaseDao getOrNull(@DaoId UUID id) {
        return back.getOrNull(id);
    }

    protected <T extends BaseDao> T get(@DaoId UUID id, Class<T> type) {
        try {
            BaseDao ret = back.getOrNull(id);
            if (ret == null) {
                throw new WebApplicationException(type.getSimpleName() + " not found (id=" + id + ")",
                        Response.Status.NOT_FOUND);
            }
            if (ret.getClass() != type) {
                throw new WebApplicationException(type.getSimpleName() + " of the wrong type (id=" + id + ", actual=" + ret.getClass().getSimpleName() + ", expected=" + type.getSimpleName() + ")",
                        Response.Status.NOT_FOUND);
            }
            BaseDao.assertStillMutable(ret);
            return (T)ret;
        } catch (ClassCastException ex) {
            throw new WebApplicationException(type.getSimpleName() + " of the wrong type (id=" + id + ")",
                    ex, Response.Status.NOT_FOUND);
        }
    }

    protected BaseDao get(@DaoId UUID id) {
        BaseDao ret = back.getOrNull(id);
        if (ret == null) {
            throw new WebApplicationException("Object data (id=" + id + ") not found",
                    Response.Status.NOT_FOUND);
        }
        return ret;
    }

    public BaseDao getExceptional(@DaoId UUID id) {
        return this.get(id);
    }

    public DataContainer getRaw(@DaoId UUID id)
    {
        DataContainer ret = back.getRawOrNull(id);
        if (ret == null) {
            throw new WebApplicationException("Object data (id=" + id + ") not found",
                    Response.Status.NOT_FOUND);
        }
        return ret;
    }

    @Override
    public @Nullable DataContainer getRawOrNull(@DaoId UUID id)
    {
        return back.getRawOrNull(id);
    }

    @Override
    public <T extends BaseDao> Iterable<MessageMetaDto> getHistory(@DaoId UUID id, Class<T> clazz) {
        return back.getHistory(id, clazz);
    }

    @Override
    public @Nullable BaseDao getVersionOrNull(@DaoId UUID id, MessageMetaDto meta) {
        return back.getVersionOrNull(id, meta);
    }

    @Override
    public @Nullable MessageDataDto getVersionMsgOrNull(@DaoId UUID id, MessageMetaDto meta) {
        return back.getVersionMsgOrNull(id, meta);
    }

    public BaseDao getVersion(@DaoId UUID id, MessageMetaDto meta) {
        BaseDao ret = back.getVersionOrNull(id, meta);
        if (ret == null) {
            throw new WebApplicationException("Object version data (id=" + id + ") not found",
                    Response.Status.NOT_FOUND);
        }
        return ret;
    }

    public MessageDataDto getVersionMsg(@DaoId UUID id, MessageMetaDto meta) {
        MessageDataDto ret = back.getVersionMsgOrNull(id, meta);
        if (ret == null) {
            throw new WebApplicationException("Object version message (id=" + id + ") not found",
                    Response.Status.NOT_FOUND);
        }
        return ret;
    }

    @Override
    public Set<BaseDao> getAll() {
        return back.getAll();
    }

    @Override
    public <T extends BaseDao> Set<T> getAll(Class<T> type) {
        return back.getAll(type);
    }

    @Override
    public <T extends BaseDao> List<DataContainer> getAllRaw() { return back.getAllRaw(); }

    @Override
    public <T extends BaseDao> List<DataContainer> getAllRaw(Class<T> type) { return back.getAllRaw(type); }

    @Override
    public <T extends BaseDao> List<T> getMany(Collection<@DaoId UUID> ids, Class<T> type) {
        return back.getMany(ids, type);
    }
}
