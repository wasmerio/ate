package com.tokera.ate.io.layers;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.io.api.IAteIO;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.DataTransaction;
import com.tokera.ate.units.Hash;
import com.tokera.ate.io.repo.DataContainer;
import com.tokera.ate.io.repo.DataSubscriber;
import org.checkerframework.checker.nullness.qual.Nullable;
import java.util.*;
import java.util.function.Predicate;

/**
 * IO system that chains two IO subsystems together where the upper data takes preference over the lower
 */
final public class SplitIO implements IAteIO {

    private final IAteIO upper;
    private final IAteIO lower;
    private final Random rand = new Random();

    public SplitIO(IAteIO upper, IAteIO lower) {
        this.upper = upper;
        this.lower = lower;
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
    public @Nullable MessageDataHeaderDto readRootOfTrust(PUUID id) {
        MessageDataHeaderDto ret = upper.readRootOfTrust(id);
        if (ret != null) return ret;
        return lower.readRootOfTrust(id);
    }

    @Override
    final public @Nullable BaseDao readOrNull(PUUID id) {
        BaseDao ret = this.upper.readOrNull(id);
        if (ret != null) return ret;
        return lower.readOrNull(id);
    }

    @Override
    final public BaseDao readOrThrow(PUUID id) {
        BaseDao ret = this.upper.readOrNull(id);
        if (ret != null) return ret;
        return lower.readOrThrow(id);
    }

    @Override
    final public @Nullable DataContainer readRawOrNull(PUUID id) {
        DataContainer ret = this.upper.readRawOrNull(id);
        if (ret != null) return ret;
        return lower.readRawOrNull(id);
    }

    @Override
    final public <T extends BaseDao> Iterable<MessageMetaDto> readHistory(PUUID id, Class<T> clazz) {
        return lower.readHistory(id, clazz);
    }

    @Override
    final public @Nullable BaseDao readVersionOrNull(PUUID id, long offset) {
        BaseDao ret = upper.readVersionOrNull(id, offset);
        if (ret != null) return ret;
        return lower.readVersionOrNull(id, offset);
    }

    @Override
    final public @Nullable MessageDataMetaDto readVersionMsgOrNull(PUUID id, long offset) {
        MessageDataMetaDto ret = upper.readVersionMsgOrNull(id, offset);
        if (ret != null) return ret;
        return lower.readVersionMsgOrNull(id, offset);
    }

    @Override
    final public List<BaseDao> view(IPartitionKey partitionKey, Predicate<BaseDao> predicate) {
        List<BaseDao> ret = lower.view(partitionKey, predicate);

        for (BaseDao entity : upper.view(partitionKey, predicate)) {
            ret.add(entity);
        }

        return ret;
    }

    @Override
    final public <T extends BaseDao> List<T> view(IPartitionKey partitionKey, Class<T> type, Predicate<T> predicate) {
        List<T> ret = lower.view(partitionKey, type, predicate);

        for (T entity : upper.view(partitionKey, type, predicate)) {
            ret.add(entity);
        }

        return ret;
    }

    @Override
    final public <T extends BaseDao> List<DataContainer> readAllRaw(IPartitionKey partitionKey) {
        return lower.readAllRaw(partitionKey);
    }

    @Override
    final public <T extends BaseDao> List<DataContainer> readAllRaw(IPartitionKey partitionKey, Class<T> type) {
        return lower.readAllRaw(partitionKey, type);
    }

    @Override
    final public @Nullable MessagePublicKeyDto publicKeyOrNull(IPartitionKey partitionKey, @Hash String hash) {
        MessagePublicKeyDto ret = upper.publicKeyOrNull(partitionKey, hash);
        if (ret != null) return ret;
        return lower.publicKeyOrNull(partitionKey, hash);
    }

    @Override
    public void send(DataTransaction transaction, boolean validate) {
        upper.send(transaction, validate);
        lower.send(transaction, false);
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
    public MessageSyncDto beginSync(IPartitionKey partitionKey, MessageSyncDto sync) {
        upper.beginSync(partitionKey, sync);
        lower.beginSync(partitionKey, sync);
        return sync;
    }

    @Override
    final public boolean finishSync(IPartitionKey partitionKey, MessageSyncDto sync) {
        upper.finishSync(partitionKey, sync);
        return lower.finishSync(partitionKey, sync);
    }

    @Override
    public DataSubscriber backend() {
        return lower.backend();
    }
}
