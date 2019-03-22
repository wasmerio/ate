package com.tokera.ate.dao.io;

import com.tokera.server.api.dao.BaseDao;
import com.tokera.server.api.delegate.MegaDelegate;
import com.tokera.server.api.dto.EffectivePermissions;
import com.tokera.server.api.dto.msg.*;
import com.tokera.server.api.qualifiers.LoggingEngine;
import com.tokera.server.api.qualifiers.StorageSystem;
import com.tokera.server.api.repositories.*;
import com.tokera.server.api.units.ClassName;
import com.tokera.server.api.units.DaoId;
import com.tokera.server.api.units.Hash;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.RequestScoped;
import javax.inject.Inject;
import java.util.*;
import java.util.stream.Collectors;

/**
 * Records reads and writes to data objects - this hook is used for the ATE caching engine
 * @author johnathan.sharratt@gmail.com
 */
@RequestScoped
@LoggingEngine
public class AccessLogIO implements ICloudIO {

    private MegaDelegate d = MegaDelegate.getUnsafe();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    @StorageSystem
    private ICloudIO back;
    
    private final Map<String, Integer> readClazzCnts = new HashMap<>();
    private final Map<String, Integer> wroteClazzCnts = new HashMap<>();
    private final Set<String> readRecords = new HashSet<>();
    private final Set<String> wroteRecords = new HashSet<>();
    private boolean isPaused = false;
    
    private final int max_items_per_clazz = 10;

    public AccessLogIO() {
    }
    
    public <T extends BaseDao> void recordRead(Class<T> clazz) {
        if (isPaused == true) return;
        String clazzName = clazz.getSimpleName();
        String clazzNameSep = clazzName + ":";
        
        Integer cnt = readClazzCnts.getOrDefault(clazzName, 0);
        if (cnt > 0 && cnt < Integer.MAX_VALUE) {
            readRecords.removeAll(readRecords.stream()
                    .filter(r -> r.startsWith(clazzNameSep))
                    .collect(Collectors.toSet()));
        }
        
        readRecords.add(clazzNameSep + "*");
        readClazzCnts.put(clazzName, Integer.MAX_VALUE);
    }
    
    public <T extends BaseDao> void recordWrote(Class<T> clazz) {
        if (isPaused == true) return;
        String clazzName = clazz.getSimpleName();
        String clazzNameSep = clazzName + ":";
        
        Integer cnt = wroteClazzCnts.getOrDefault(clazzName, 0);
        if (cnt > 0 && cnt < Integer.MAX_VALUE) {
            wroteRecords.removeAll(wroteRecords.stream()
                    .filter(r -> r.startsWith(clazzNameSep))
                    .collect(Collectors.toSet()));
        }
        
        wroteRecords.add(clazzNameSep + "*");
        wroteClazzCnts.put(clazzName, Integer.MAX_VALUE);
    }
    
    public void recordRead(@DaoId UUID id, Class<?> clazz) {
        if (isPaused == true) return;
        String clazzName = clazz.getSimpleName();
        String clazzNameSep = clazzName + ":";
        
        Integer cnt = readClazzCnts.getOrDefault(clazzName, 0);
        if (cnt >= max_items_per_clazz && cnt < Integer.MAX_VALUE) {
            readRecords.removeAll(readRecords.stream()
                    .filter(r -> r.startsWith(clazzNameSep))
                    .collect(Collectors.toSet()));
            
            readRecords.add(clazzNameSep + "*");
            readClazzCnts.put(clazzName, Integer.MAX_VALUE);
        }
        
        if (readRecords.add(clazz.getSimpleName() + ":" + id) == true) {
            readClazzCnts.put(clazzName, cnt + 1);
        }
    }
    
    public void recordWrote(@DaoId UUID id, Class<?> clazz) {
        if (isPaused == true) return;
        String clazzName = clazz.getSimpleName();
        String clazzNameSep = clazzName + ":";
        
        Integer cnt = wroteClazzCnts.getOrDefault(clazzName, 0);
        if (cnt >= max_items_per_clazz && cnt < Integer.MAX_VALUE) {
            wroteRecords.removeAll(wroteRecords.stream()
                    .filter(r -> r.startsWith(clazzNameSep))
                    .collect(Collectors.toSet()));
            
            wroteRecords.add(clazzNameSep + "*");
            wroteClazzCnts.put(clazzName, Integer.MAX_VALUE);
        }
        
        if (wroteRecords.add(clazz.getSimpleName() + ":" + id) == true) {
            wroteClazzCnts.put(clazzName, cnt + 1);
        }
    }
    
    public Set<@Hash String> getReadRecords() {
        return this.readRecords;
    }
    
    public Set<@Hash String> getWroteRecords() {
        return this.wroteRecords;
    }
    
    @Override
    public boolean merge(BaseDao t) {
        boolean ret = back.merge(t);
        if (ret == false) return false;
        this.recordWrote(t.getId(), t.getClass());
        return true;
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
        this.recordWrote(t.getId(), t.getClass());
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
        this.recordWrote(t.getId(), t.getClass());
        return back.remove(t);
    }
    
    @Override
    public void removeLater(BaseDao t) {
        this.recordWrote(t.getId(), t.getClass());
        back.removeLater(t);
    }
    
    @Override
    public boolean remove(@DaoId UUID id, Class<?> type) {
        this.recordWrote(id, type);
        return back.remove(id, type);
    }

    @Override
    public void cache(BaseDao entity) {
        back.cache(entity);
    }
    
    @Override
    public EffectivePermissions perms(@DaoId UUID id, @Nullable @DaoId UUID parentId, boolean usePostMerged) {
        return back.perms(id, parentId, usePostMerged);
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
    public @Nullable MessagePublicKeyDto publicKeyOrNull(@Hash String hash) {
        return back.publicKeyOrNull(hash);
    }
    
    @Override
    public boolean exists(@Nullable @DaoId UUID _id) {
        @DaoId UUID id = _id;
        if (id == null) return false;
        return back.exists(id);
    }
    
    @Override
    public boolean ethereal() {
        return back.ethereal();
    }
    
    @Override
    public boolean everExisted(@Nullable @DaoId UUID _id) {
        @DaoId UUID id = _id;
        if (id == null) return false;
        return back.everExisted(id);
    }
    
    @Override
    public boolean immutable(@DaoId UUID id) {
        return back.immutable(id);
    }

    @Override
    public @Nullable BaseDao getOrNull(@DaoId UUID id) {
        BaseDao ret = back.getOrNull(id);
        if (ret != null) {
            this.recordRead(id, ret.getClass());
        }
        return ret;
    }

    @Override
    public @Nullable DataContainer getRawOrNull(@DaoId UUID id) { return back.getRawOrNull(id); }
    
    @Override
    public @Nullable BaseDao getVersionOrNull(@DaoId UUID id, MessageMetaDto meta) {
        return back.getVersionOrNull(id, meta);
    }
    
    @Override
    public @Nullable MessageDataDto getVersionMsgOrNull(@DaoId UUID id, MessageMetaDto meta) {
        return back.getVersionMsgOrNull(id, meta);
    }
    
    @Override
    public <T extends BaseDao> Iterable<MessageMetaDto> getHistory(@DaoId UUID id, Class<T> clazz) {
        this.recordRead(id, clazz);
        return back.getHistory(id, clazz);
    }

    @Override
    public <T extends BaseDao> Set<T> getAll() {
        return back.getAll();
    }

    @Override
    public <T extends BaseDao> Set<T> getAll(Class<T> type) {
        Set<T> ret = back.getAll(type);
        this.recordRead(type);
        return ret;
    }

    @Override
    public <T extends BaseDao> List<DataContainer> getAllRaw() {
        return back.getAllRaw();
    }

    @Override
    public <T extends BaseDao> List<DataContainer> getAllRaw(Class<T> type) {
        return back.getAllRaw(type);
    }
    
    @Override
    public <T extends BaseDao> List<T> getMany(Collection<@DaoId UUID> ids, Class<T> type) {
        List<T> ret = back.getMany(ids, type);
        for (T entity : ret) {
            this.recordRead(entity.getId(), type);
        }
        return ret;
    }
    
    public void pause() {
        isPaused = true;
    }
    
    public void unpause() {
        isPaused = false;
    }
}
