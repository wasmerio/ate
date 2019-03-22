package com.tokera.ate.dao.io;

import com.tokera.server.api.dao.BaseDao;
import com.tokera.server.api.dao.IRoles;
import com.tokera.server.api.dao.msg.MessageBaseSerializer;
import com.tokera.server.api.delegate.MegaDelegate;
import com.tokera.server.api.dto.EffectivePermissions;
import com.tokera.server.api.dto.msg.*;
import com.tokera.server.api.qualifiers.CachingSystem;
import com.tokera.server.api.repositories.DataContainer;
import com.tokera.server.api.units.DaoId;
import com.tokera.server.api.units.Hash;
import org.checkerframework.checker.nullness.qual.Nullable;
import sun.reflect.generics.reflectiveObjects.NotImplementedException;

import javax.enterprise.context.RequestScoped;
import java.util.*;
import java.util.stream.Collectors;

@RequestScoped
@CachingSystem
public class CacheIO implements ICloudIO
{
    private MegaDelegate d = MegaDelegate.getUnsafe();

    private class TopicCache {
        public final Map<UUID, BaseDao> entries = new HashMap<>();
        public final Map<String, MessagePublicKeyDto> publicKeys = new HashMap<>();
        public final Map<String, MessageEncryptTextDto> encryptTexts = new HashMap<>();
    }

    protected Map<String, TopicCache> cache = new HashMap<>();
    
    public CacheIO() {
    }

    protected TopicCache getTopicCache() {
        String topic = d.contextShare.getTopicName();

        if (this.cache.containsKey(topic) == true) {
            return this.cache.get(topic);
        }

        TopicCache ret = new TopicCache();
        this.cache.put(topic, ret);
        return ret;
    }

    @Override
    public boolean merge(BaseDao entity) {
        TopicCache c = this.getTopicCache();
        c.entries.put(entity.getId(), entity);
        return true;
    }

    @Override
    public boolean merge(MessagePublicKeyDto t) {
        TopicCache c = this.getTopicCache();
        c.publicKeys.put(MessageBaseSerializer.getKey(t), t);
        return true;
    }

    @Override
    public boolean merge(MessageEncryptTextDto t) {
        TopicCache c = this.getTopicCache();
        c.encryptTexts.put(MessageBaseSerializer.getKey(t), t);
        return true;
    }

    @Override
    public void mergeLater(BaseDao entity) {
        merge(entity);
    }

    @Override
    public boolean remove(BaseDao entity) {
        return remove(entity.getId(), entity.getClass());
    }

    @Override
    public boolean remove(@DaoId UUID id, Class<?> type) {
        TopicCache c = this.getTopicCache();
        return c.entries.remove(id) != null;
    }

    @Override
    public void removeLater(BaseDao entity) {
        remove(entity);
    }

    @Override
    public void cache(BaseDao entity) {
        merge(entity);
    }

    @Override
    public boolean exists(@Nullable @DaoId UUID _id) {
        @DaoId UUID id = _id;
        if (id == null) return false;

        TopicCache c = this.getTopicCache();
        return c.entries.containsKey(id);
    }

    @Override
    public boolean ethereal() {
        return false;
    }

    @Override
    public boolean everExisted(@Nullable @DaoId UUID _id) {
        @DaoId UUID id = _id;
        if (id == null) return false;
        return exists(id);
    }

    @Override
    public boolean immutable(@DaoId UUID id) {
        return false;
    }

    @Override
    public EffectivePermissions perms(@DaoId UUID origId, @Nullable @DaoId UUID origParentId, boolean usePostMerged) {
        EffectivePermissions ret = new EffectivePermissions();

        boolean isParents = false;
        boolean inheritRead = true;
        boolean inheritWrite = true;

        @DaoId UUID id = origId;
        @DaoId UUID parentId = origParentId;
        while (id != null)
        {
            BaseDao obj = this.getOrNull(id);
            if (obj != null) {
                ret.updateEncryptKeyFromObjIfNull(obj);

                if (obj instanceof IRoles) {
                    IRoles roles = (IRoles) obj;

                    if (inheritRead == true) {
                        for (String p : roles.getTrustAllowRead().values()) {
                            if (ret.rolesRead.contains(p) == false) {
                                ret.rolesRead.add(p);
                            }
                        }
                        if (isParents) {
                            for (String p : roles.getTrustAllowRead().values()) {
                                if (ret.anchorRolesRead.contains(p) == false) {
                                    ret.anchorRolesRead.add(p);
                                }
                            }
                        }
                    }
                    if (inheritWrite == true) {
                        for (String p : roles.getTrustAllowWrite().values()) {
                            if (ret.rolesWrite.contains(p) == false) {
                                ret.rolesWrite.add(p);
                            }
                        }
                        if (isParents) {
                            for (String p : roles.getTrustAllowWrite().values()) {
                                if (ret.anchorRolesWrite.contains(p) == false) {
                                    ret.anchorRolesWrite.add(p);
                                }
                            }
                        }
                    }
                    if (roles.getTrustInheritRead() == false) {
                        inheritRead = false;
                    }
                    if (roles.getTrustInheritWrite() == false &&
                            this.exists(id) == true) {
                        inheritWrite = false;
                    }
                }
                parentId = obj.getParentId();
            }

            isParents = true;
            id = parentId;
            parentId = null;
        }

        return ret;
    }

    @Override
    public @Nullable BaseDao getOrNull(@DaoId UUID id) {
        TopicCache c = this.getTopicCache();
        if (c.entries.containsKey(id) == false) return null;
        return c.entries.get(id);
    }

    @Override
    public @Nullable DataContainer getRawOrNull(@DaoId UUID id) {
        throw new NotImplementedException();
    }

    @Override
    public <T extends BaseDao> Iterable<MessageMetaDto> getHistory(@DaoId UUID id, Class<T> clazz) {
        throw new NotImplementedException();
    }

    @Override
    public BaseDao getVersionOrNull(@DaoId UUID id, MessageMetaDto meta) {
        throw new NotImplementedException();
    }

    @Override
    public MessageDataDto getVersionMsgOrNull(@DaoId UUID id, MessageMetaDto meta) {
        throw new NotImplementedException();
    }

    @Override
    public <T extends BaseDao> Set<T> getAll() {
        TopicCache c = this.getTopicCache();
        return c.entries.values()
                .stream()
                .map(e -> (T)e)
                .collect(Collectors.toSet());
    }

    @Override
    public <T extends BaseDao> Set<T> getAll(Class<T> type) {
        TopicCache c = this.getTopicCache();
        return c.entries.values()
                .stream()
                .filter(e -> e.getClass() == type)
                .map(e -> (T)e)
                .collect(Collectors.toSet());
    }

    @Override
    public <T extends BaseDao> List<DataContainer> getAllRaw() {
        throw new NotImplementedException();
    }

    @Override
    public <T extends BaseDao> List<DataContainer> getAllRaw(Class<T> type) {
        throw new NotImplementedException();
    }

    @Override
    public <T extends BaseDao> List<T> getMany(Collection<@DaoId UUID> ids, Class<T> type) {
        List<T> ret = new LinkedList();
        for (UUID id : ids) {
            @Nullable BaseDao entity = this.getOrNull(id);
            if (entity == null) continue;
            if (entity.getClass() == type) {
                ret.add((T)entity);
            }
        }
        return ret;
    }

    @Override
    public @Nullable MessagePublicKeyDto publicKeyOrNull(@Hash String hash) {
        TopicCache c = this.getTopicCache();
        if (c.publicKeys.containsKey(hash) == false) return null;
        return c.publicKeys.get(hash);
    }

    @Override
    public void mergeDeferred() {
    }

    @Override
    public void clearDeferred() {
    }

    @Override
    public void clearCache(@DaoId UUID id) {
        cache.remove(id);
    }

    @Override
    public void warm() {
    }

    @Override
    public void sync() {
    }

    @Override
    public boolean sync(MessageSyncDto sync) {
        return true;
    }
}