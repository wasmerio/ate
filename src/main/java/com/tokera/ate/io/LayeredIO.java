package com.tokera.ate.io;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.msg.*;
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
final public class LayeredIO implements IAteIO {

    private final IAteIO upper;
    private final IAteIO lower;

    public LayeredIO(IAteIO upper, IAteIO lower) {
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
    final public boolean mergeAsyncWithoutValidation(BaseDao entity) {
        boolean ret = lower.mergeAsyncWithoutValidation(entity);
        upper.mergeAsyncWithoutValidation(entity);
        return ret;
    }

    @Override
    final public boolean merge(MessagePublicKeyDto publicKey) {
        boolean ret = lower.merge(publicKey);
        upper.merge(publicKey);
        return ret;
    }

    @Override
    final public boolean merge(MessageEncryptTextDto encryptText) {
        boolean ret = lower.merge(encryptText);
        upper.merge(encryptText);
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
    final public boolean remove(@DaoId UUID id, Class<?> type) {
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
    final public boolean exists(@Nullable @DaoId UUID id) {
        return upper.exists(id) || lower.exists(id);
    }

    @Override
    final public boolean ethereal() {
        return upper.ethereal() || lower.ethereal();
    }

    @Override
    final public boolean everExisted(@Nullable @DaoId UUID id) {
        return upper.everExisted(id) || lower.everExisted(id);
    }

    @Override
    final public boolean immutable(@DaoId UUID id) {
        return upper.immutable(id) || lower.immutable(id);
    }

    @Override
    public @Nullable MessageDataHeaderDto getRootOfTrust(UUID id) {
        MessageDataHeaderDto ret = upper.getRootOfTrust(id);
        if (ret != null) return ret;
        return lower.getRootOfTrust(id);
    }

    @Override
    final public @Nullable BaseDao getOrNull(@DaoId UUID id) {
        BaseDao ret = this.upper.getOrNull(id);
        if (ret != null) return ret;

        ret = lower.getOrNull(id);

        if (ret != null) {
            this.upper.mergeLaterWithoutValidation(ret);
        }
        return ret;
    }

    @Override
    final public <T extends BaseDao> List<T> getMany(Collection<@DaoId UUID> ids, Class<T> type) {
        List<T> first = upper.getMany(ids, type);
        if (first.size() == ids.size()) {
            return first;
        }

        Map<@DaoId UUID, T> found = first.stream().collect(Collectors.toMap(a -> a.getId(), b -> b));
        List<@DaoId UUID> left = ids.stream().filter(a -> found.containsKey(a) == false).collect(Collectors.toList());

        List<T> more = lower.getMany(left, type);
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
    final public @Nullable DataContainer getRawOrNull(@DaoId UUID id) {
        DataContainer ret = this.upper.getRawOrNull(id);
        if (ret != null) return ret;
        return lower.getRawOrNull(id);
    }

    @Override
    final public <T extends BaseDao> Iterable<MessageMetaDto> getHistory(@DaoId UUID id, Class<T> clazz) {
        return lower.getHistory(id, clazz);
    }

    @Override
    final public @Nullable BaseDao getVersionOrNull(@DaoId UUID id, MessageMetaDto meta) {
        BaseDao ret = upper.getVersionOrNull(id, meta);
        if (ret != null) return ret;
        return lower.getVersionOrNull(id, meta);
    }

    @Override
    final public @Nullable MessageDataDto getVersionMsgOrNull(@DaoId UUID id, MessageMetaDto meta) {
        MessageDataDto ret = upper.getVersionMsgOrNull(id, meta);
        if (ret != null) return ret;
        return lower.getVersionMsgOrNull(id, meta);
    }

    @Override
    final public Set<BaseDao> getAll() {
        Set<BaseDao> ret = lower.getAll();

        for (BaseDao entity : upper.getAll()) {
            ret.add(entity);
        }

        return ret;
    }

    @Override
    final public <T extends BaseDao> Set<T> getAll(Class<T> type) {
        Set<T> ret = lower.getAll(type);

        for (T entity : upper.getAll(type)) {
            ret.add(entity);
        }

        return ret;
    }

    @Override
    final public <T extends BaseDao> List<DataContainer> getAllRaw() {
        return lower.getAllRaw();
    }

    @Override
    final public <T extends BaseDao> List<DataContainer> getAllRaw(Class<T> type) {
        return lower.getAllRaw(type);
    }

    @Override
    final public @Nullable MessagePublicKeyDto publicKeyOrNull(@Hash String hash) {
        MessagePublicKeyDto ret = upper.publicKeyOrNull(hash);
        if (ret != null) return ret;
        return lower.publicKeyOrNull(hash);
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
    final public void clearCache(@DaoId UUID id) {
        lower.clearCache(id);
        upper.clearCache(id);
    }

    @Override
    final public void warm() {
        upper.warm();
        lower.warm();
    }

    @Override
    final public void sync() {
        upper.sync();
        lower.sync();
    }

    @Override
    final public boolean sync(MessageSyncDto sync) {
        upper.sync(sync);
        return lower.sync(sync);
    }

    @Override
    public DataSubscriber backend() {
        return lower.backend();
    }
}
