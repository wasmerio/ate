package com.tokera.ate.io.layers;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.base.BaseDaoInternal;
import com.tokera.ate.dao.kafka.MessageSerializer;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.io.api.IAteIO;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.core.PartitionKeyComparator;
import com.tokera.ate.io.repo.DataContainer;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;
import com.tokera.ate.io.repo.DataSubscriber;
import org.apache.commons.lang.NotImplementedException;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.RequestScoped;
import java.util.*;
import java.util.stream.Collectors;

/**
 * IO system that stores the data objects in memory for the duration of the currentRights scope
 */
@RequestScoped
public class MemoryRequestCacheIO implements IAteIO
{
    private AteDelegate d = AteDelegate.get();

    private class PartitionCache {
        public final Map<UUID, BaseDao> entries = new HashMap<>();
        public final Map<String, MessagePublicKeyDto> publicKeys = new HashMap<>();
        public final Map<String, MessageSecurityCastleDto> castles = new HashMap<>();
    }

    protected Map<IPartitionKey, PartitionCache> cache = new TreeMap<>(new PartitionKeyComparator());

    public MemoryRequestCacheIO() {
    }

    protected PartitionCache getPartitionCache(IPartitionKey partitionKey) {
        if (this.cache.containsKey(partitionKey) == true) {
            return this.cache.get(partitionKey);
        }

        PartitionCache ret = new PartitionCache();
        this.cache.put(partitionKey, ret);
        return ret;
    }

    @Override
    public boolean merge(BaseDao entity) {
        PartitionCache c = this.getPartitionCache(entity.partitionKey());
        c.entries.put(entity.getId(), entity);
        return true;
    }

    @Override
    public boolean merge(IPartitionKey partitionKey, MessagePublicKeyDto t) {
        PartitionCache c = this.getPartitionCache(partitionKey);
        c.publicKeys.put(MessageSerializer.getKey(t), t);
        return true;
    }

    @Override
    public boolean merge(IPartitionKey partitionKey, MessageSecurityCastleDto t) {
        PartitionCache c = this.getPartitionCache(partitionKey);
        c.castles.put(MessageSerializer.getKey(t), t);
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
        for (BaseDao entity : entities) {
            PartitionCache c = this.getPartitionCache(entity.partitionKey());
            c.entries.put(entity.getId(), entity);
        }
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
        return remove(entity.addressableId(), entity.getClass());
    }

    @Override
    public boolean remove(PUUID id, Class<?> type) {
        PartitionCache c = this.getPartitionCache(id.partition());
        return c.entries.remove(id) != null;
    }

    @Override
    public void removeLater(BaseDao entity)
    {
        PartitionCache c = this.getPartitionCache(entity.partitionKey());
        c.entries.remove(entity.getId());
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
    public boolean exists(@Nullable PUUID _id) {
        @DaoId PUUID id = _id;
        if (id == null) return false;
        PartitionCache c = this.getPartitionCache(id.partition());
        return c.entries.containsKey(id);
    }

    @Override
    public boolean ethereal(IPartitionKey partitionKey) {
        return false;
    }

    @Override
    public boolean everExisted(@Nullable PUUID _id) {
        @DaoId PUUID id = _id;
        if (id == null) return false;
        return exists(id);
    }

    @Override
    public boolean immutable(@DaoId PUUID id) {
        return false;
    }

    @Override
    public @Nullable MessageDataHeaderDto getRootOfTrust(PUUID id) {
        return null;
    }

    public @Nullable BaseDao getOrNull(@DaoId UUID id) {
        for (PartitionCache c : this.cache.values()) {
            if (c.entries.containsKey(id) == false) continue;
            BaseDao ret = c.entries.get(id);
            BaseDaoInternal.assertStillMutable(ret);
            return ret;
        }
        return null;
    }

    @Override
    public @Nullable BaseDao getOrNull(PUUID id) {
        PartitionCache c = this.getPartitionCache(id.partition());
        if (c.entries.containsKey(id.id()) == false) return null;
        BaseDao ret = c.entries.get(id.id());
        BaseDaoInternal.assertStillMutable(ret);
        return ret;
    }

    @Override
    public BaseDao getOrThrow(PUUID id) {
        PartitionCache c = this.getPartitionCache(id.partition());
        if (c.entries.containsKey(id.id()) == false) {
            throw new RuntimeException("Failed to find a data object of id [" + id + "]");
        }
        BaseDao ret = c.entries.get(id.id());
        BaseDaoInternal.assertStillMutable(ret);
        return ret;
    }

    @Override
    public @Nullable DataContainer getRawOrNull(PUUID id) {
        return null;
    }

    @Override
    public <T extends BaseDao> Iterable<MessageMetaDto> getHistory(PUUID id, Class<T> clazz) {
        throw new NotImplementedException();
    }

    @Override
    public @Nullable BaseDao getVersionOrNull(PUUID id, MessageMetaDto meta) {
        return null;
    }

    @Override
    public @Nullable MessageDataDto getVersionMsgOrNull(PUUID id, MessageMetaDto meta) {
        return null;
    }

    @Override
    public Set<BaseDao> getAll(IPartitionKey partitionKey) {
        PartitionCache c = this.getPartitionCache(partitionKey);
        return c.entries.values()
                .stream()
                .collect(Collectors.toSet());
    }

    @SuppressWarnings({"unchecked"})
    @Override
    public <T extends BaseDao> Set<T> getAll(IPartitionKey partitionKey, Class<T> type) {
        PartitionCache c = this.getPartitionCache(partitionKey);
        return c.entries.values()
                .stream()
                .filter(e -> e.getClass() == type)
                .map(e -> (T)e)
                .collect(Collectors.toSet());
    }

    @Override
    public <T extends BaseDao> List<DataContainer> getAllRaw(IPartitionKey partitionKey) {
        throw new NotImplementedException();
    }

    @Override
    public <T extends BaseDao> List<DataContainer> getAllRaw(IPartitionKey partitionKey, Class<T> type) {
        throw new NotImplementedException();
    }

    @SuppressWarnings({"unchecked"})
    @Override
    public <T extends BaseDao> List<T> getMany(IPartitionKey partitionKey, Iterable<@DaoId UUID> ids, Class<T> type) {
        List<T> ret = new LinkedList<>();
        for (UUID id : ids) {
            @Nullable BaseDao entity = this.getOrNull(PUUID.from(partitionKey, id));
            if (entity == null) continue;
            if (entity.getClass() == type) {
                ret.add((T)entity);
            }
        }
        return ret;
    }

    @Override
    public @Nullable MessagePublicKeyDto publicKeyOrNull(IPartitionKey partitionKey, @Hash String hash) {
        PartitionCache c = this.getPartitionCache(partitionKey);
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
    public void clearCache(PUUID id) {
        PartitionCache c = this.getPartitionCache(id.partition());
        c.entries.remove(id.id());
    }

    @Override
    public void warm(IPartitionKey partitionKey) {
    }

    @Override
    public void sync(IPartitionKey partitionKey) {
    }

    @Override
    public boolean sync(IPartitionKey partitionKey, MessageSyncDto sync) {
        return true;
    }

    @Override
    public DataSubscriber backend() {
        throw new NotImplementedException();
    }
}