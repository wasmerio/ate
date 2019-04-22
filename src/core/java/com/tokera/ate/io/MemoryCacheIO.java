package com.tokera.ate.io;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.kafka.MessageSerializer;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.io.repo.DataContainer;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;
import com.tokera.ate.io.repo.DataSubscriber;
import org.checkerframework.checker.nullness.qual.Nullable;
import sun.reflect.generics.reflectiveObjects.NotImplementedException;

import javax.enterprise.context.RequestScoped;
import java.util.*;
import java.util.stream.Collectors;

/**
 * IO system that stores the data objects in memory for the duration of the currentRights scope
 */
@RequestScoped
public class MemoryCacheIO implements IAteIO
{
    private AteDelegate d = AteDelegate.getUnsafe();

    private class TopicCache {
        public final Map<UUID, BaseDao> entries = new HashMap<>();
        public final Map<String, MessagePublicKeyDto> publicKeys = new HashMap<>();
        public final Map<String, MessageEncryptTextDto> encryptTexts = new HashMap<>();
    }

    protected Map<String, TopicCache> cache = new HashMap<>();
    
    public MemoryCacheIO() {
    }

    protected TopicCache getTopicCache() {
        String topic = d.requestContext.getCurrentTopicScope();

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
    public boolean mergeAsync(BaseDao entity) {
        return merge(entity);
    }

    @Override
    public boolean mergeWithoutValidation(BaseDao entity) {
        return merge(entity);
    }

    @Override
    public boolean mergeAsyncWithoutValidation(BaseDao entity) {
        return merge(entity);
    }

    public <T extends BaseDao> boolean mergeMany(Iterable<T> entities) {
        TopicCache c = this.getTopicCache();
        for (BaseDao entity : entities) {
            c.entries.put(entity.getId(), entity);
        }
        return true;
    }

    @Override
    public boolean merge(MessagePublicKeyDto t) {
        TopicCache c = this.getTopicCache();
        c.publicKeys.put(MessageSerializer.getKey(t), t);
        return true;
    }

    @Override
    public boolean merge(MessageEncryptTextDto t) {
        TopicCache c = this.getTopicCache();
        c.encryptTexts.put(MessageSerializer.getKey(t), t);
        return true;
    }

    @Override
    public void mergeLater(BaseDao entity) {
        merge(entity);
    }

    @Override
    public void mergeLaterWithoutValidation(BaseDao entity) {
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
    public void decache(BaseDao entity) {
        remove(entity);
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
    public @Nullable MessageDataHeaderDto getRootOfTrust(UUID id) {
        return null;
    }

    @Override
    public @Nullable BaseDao getOrNull(@DaoId UUID id) {
        TopicCache c = this.getTopicCache();
        if (c.entries.containsKey(id) == false) return null;
        BaseDao ret = c.entries.get(id);
        BaseDao.assertStillMutable(ret);
        return ret;
    }

    @Override
    public @Nullable DataContainer getRawOrNull(@DaoId UUID id) {
        return null;
    }

    @Override
    public <T extends BaseDao> Iterable<MessageMetaDto> getHistory(@DaoId UUID id, Class<T> clazz) {
        throw new NotImplementedException();
    }

    @Override
    public @Nullable BaseDao getVersionOrNull(@DaoId UUID id, MessageMetaDto meta) {
        return null;
    }

    @Override
    public @Nullable MessageDataDto getVersionMsgOrNull(@DaoId UUID id, MessageMetaDto meta) {
        return null;
    }

    @Override
    public Set<BaseDao> getAll() {
        TopicCache c = this.getTopicCache();
        return c.entries.values()
                .stream()
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

    @Override
    public DataSubscriber backend() {
        throw new NotImplementedException();
    }
}