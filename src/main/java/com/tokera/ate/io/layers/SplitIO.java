package com.tokera.ate.io.layers;

import com.google.common.collect.Iterables;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.io.api.IAteIO;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;
import com.tokera.ate.io.repo.DataContainer;
import com.tokera.ate.io.repo.DataSubscriber;
import org.checkerframework.checker.nullness.qual.Nullable;
import java.util.*;
import java.util.stream.Collectors;

/**
 * IO system that chains two IO subsystems together where the upper data takes preference over the lower
 */
final public class SplitIO implements IAteIO {

    private final IAteIO upper;
    private final IAteIO lower;

    public SplitIO(IAteIO upper, IAteIO lower) {
        this.upper = upper;
        this.lower = lower;
    }

    @Override
    final public boolean merge(BaseDao entity) {
        boolean ret = lower.merge(entity);
        upper.merge(entity);
        return ret;
    }

    @Override
    final public boolean merge(IPartitionKey partitionKey, MessagePublicKeyDto publicKey) {
        boolean ret = lower.merge(partitionKey, publicKey);
        upper.merge(partitionKey, publicKey);
        return ret;
    }

    @Override
    final public boolean merge(IPartitionKey partitionKey, MessageSecurityCastleDto castle) {
        boolean ret = lower.merge(partitionKey, castle);
        upper.merge(partitionKey, castle);
        return ret;
    }

    @Override
    final public boolean mergeAsync(BaseDao entity) {
        boolean ret = lower.mergeAsync(entity);
        upper.mergeAsync(entity);
        return ret;
    }

    @Override
    final public boolean mergeWithoutValidation(BaseDao entity) {
        boolean ret = lower.mergeWithoutValidation(entity);
        upper.mergeWithoutValidation(entity);
        return ret;
    }

    @Override
    final public boolean mergeWithoutSync(BaseDao entity) {
        boolean ret = lower.mergeWithoutSync(entity);
        upper.mergeWithoutSync(entity);
        return ret;
    }

    @Override
    final public boolean mergeAsyncWithoutValidation(BaseDao entity) {
        boolean ret = lower.mergeAsyncWithoutValidation(entity);
        upper.mergeAsyncWithoutValidation(entity);
        return ret;
    }

    @Override
    final public void mergeLater(BaseDao entity) {
        lower.mergeLater(entity);
        upper.mergeLaterWithoutValidation(entity);
    }

    @Override
    final public void mergeLaterWithoutValidation(BaseDao entity) {
        lower.mergeLaterWithoutValidation(entity);
        upper.mergeLaterWithoutValidation(entity);
    }

    @Override
    final public boolean remove(BaseDao entity) {
        boolean ret = lower.remove(entity);
        upper.remove(entity);
        return ret;
    }

    @Override
    final public boolean remove(PUUID id, Class<?> type) {
        boolean ret = lower.remove(id, type);
        upper.remove(id, type);
        return ret;
    }

    @Override
    final public void removeLater(BaseDao entity) {
        upper.removeLater(entity);
        lower.removeLater(entity);
    }

    @Override
    final public void cache(BaseDao entity) {
        lower.cache(entity);
        upper.cache(entity);
    }

    @Override
    final public void decache(BaseDao entity) {
        lower.decache(entity);
        upper.decache(entity);
    }

    @Override
    final public boolean exists(@Nullable PUUID id) {
        return upper.exists(id) || lower.exists(id);
    }

    @Override
    final public boolean everExisted(@Nullable PUUID id) {
        return upper.everExisted(id) || lower.everExisted(id);
    }

    @Override
    final public boolean immutable(PUUID id) {
        return upper.immutable(id) || lower.immutable(id);
    }

    @Override
    public @Nullable MessageDataHeaderDto getRootOfTrust(PUUID id) {
        MessageDataHeaderDto ret = upper.getRootOfTrust(id);
        if (ret != null) return ret;
        return lower.getRootOfTrust(id);
    }

    @Override
    final public @Nullable BaseDao getOrNull(PUUID id, boolean shouldSave) {
        BaseDao ret = this.upper.getOrNull(id, shouldSave);
        if (ret != null) return ret;

        ret = lower.getOrNull(id, shouldSave);

        if (ret != null) {
            this.upper.mergeLaterWithoutValidation(ret);
        }
        return ret;
    }

    @Override
    final public BaseDao getOrThrow(PUUID id) {
        BaseDao ret = this.upper.getOrNull(id, true);
        if (ret != null) return ret;

        ret = lower.getOrThrow(id);

        if (ret != null) {
            this.upper.mergeLaterWithoutValidation(ret);
        }
        return ret;
    }

    @Override
    final public <T extends BaseDao> List<T> getMany(IPartitionKey partitionKey, Iterable<@DaoId UUID> ids, Class<T> type) {
        List<T> first = upper.getMany(partitionKey, ids, type);
        if (first.size() == Iterables.size(ids)) {
            return first;
        }
        if (first.size() <= 0) {
            return lower.getMany(partitionKey, ids, type);
        }

        Map<@DaoId UUID, T> found = first.stream().collect(Collectors.toMap(a -> a.getId(), b -> b));
        List<@DaoId UUID> left = new ArrayList<>();
        Iterables.filter(ids, a -> found.containsKey(a) == false).forEach(left::add);

        List<T> more = lower.getMany(partitionKey, left, type);
        for (T entity : more) {
            upper.mergeLaterWithoutValidation(entity);
            found.put(entity.getId(), entity);
        }

        List<T> ret = new ArrayList<>();
        for (@DaoId UUID id : ids) {
            if (found.containsKey(id)) {
                ret.add(found.get(id));
            }
        }
        return ret;
    }

    @Override
    final public @Nullable DataContainer getRawOrNull(PUUID id) {
        DataContainer ret = this.upper.getRawOrNull(id);
        if (ret != null) return ret;
        return lower.getRawOrNull(id);
    }

    @Override
    final public <T extends BaseDao> Iterable<MessageMetaDto> getHistory(PUUID id, Class<T> clazz) {
        return lower.getHistory(id, clazz);
    }

    @Override
    final public @Nullable BaseDao getVersionOrNull(PUUID id, MessageMetaDto meta) {
        BaseDao ret = upper.getVersionOrNull(id, meta);
        if (ret != null) return ret;
        return lower.getVersionOrNull(id, meta);
    }

    @Override
    final public @Nullable MessageDataDto getVersionMsgOrNull(PUUID id, MessageMetaDto meta) {
        MessageDataDto ret = upper.getVersionMsgOrNull(id, meta);
        if (ret != null) return ret;
        return lower.getVersionMsgOrNull(id, meta);
    }

    @Override
    final public Set<BaseDao> getAll(IPartitionKey partitionKey) {
        Set<BaseDao> ret = lower.getAll(partitionKey);

        for (BaseDao entity : upper.getAll(partitionKey)) {
            ret.add(entity);
        }

        return ret;
    }

    @Override
    final public <T extends BaseDao> Set<T> getAll(IPartitionKey partitionKey, Class<T> type) {
        Set<T> ret = lower.getAll(partitionKey, type);

        for (T entity : upper.getAll(partitionKey, type)) {
            ret.add(entity);
        }

        return ret;
    }

    @Override
    final public <T extends BaseDao> Set<T> getAll(Collection<IPartitionKey> keys, Class<T> type) {
        Set<T> ret = lower.getAll(keys, type);

        for (T entity : upper.getAll(keys, type)) {
            ret.add(entity);
        }

        return ret;
    }

    @Override
    final public <T extends BaseDao> List<DataContainer> getAllRaw(IPartitionKey partitionKey) {
        return lower.getAllRaw(partitionKey);
    }

    @Override
    final public <T extends BaseDao> List<DataContainer> getAllRaw(IPartitionKey partitionKey, Class<T> type) {
        return lower.getAllRaw(partitionKey, type);
    }

    @Override
    final public @Nullable MessagePublicKeyDto publicKeyOrNull(IPartitionKey partitionKey, @Hash String hash) {
        MessagePublicKeyDto ret = upper.publicKeyOrNull(partitionKey, hash);
        if (ret != null) return ret;
        return lower.publicKeyOrNull(partitionKey, hash);
    }

    @Override
    final public void mergeDeferred() {
        lower.mergeDeferred();
        upper.mergeDeferred();
    }

    @Override
    final public void clearDeferred() {
        lower.clearDeferred();
        upper.clearDeferred();
    }

    @Override
    final public void clearCache(PUUID id) {
        lower.clearCache(id);
        upper.clearCache(id);
    }

    @Override
    final public void warm(IPartitionKey partitionKey) {
        upper.warm(partitionKey);
        lower.warm(partitionKey);
    }

    @Override
    final public void warmAndWait(IPartitionKey partitionKey) {
        upper.warmAndWait(partitionKey);
        lower.warmAndWait(partitionKey);
    }

    @Override
    final public void sync(IPartitionKey partitionKey) {
        upper.sync(partitionKey);
        lower.sync(partitionKey);
    }

    @Override
    final public boolean sync(IPartitionKey partitionKey, MessageSyncDto sync) {
        upper.sync(partitionKey, sync);
        return lower.sync(partitionKey, sync);
    }

    @Override
    public DataSubscriber backend() {
        return lower.backend();
    }
}
