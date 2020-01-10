package com.tokera.ate.io.layers;

import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.function.Predicate;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.io.api.IAteIO;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.core.RequestAccessLog;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.io.repo.DataTransaction;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;
import com.tokera.ate.io.repo.DataContainer;
import com.tokera.ate.io.repo.DataSubscriber;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.inject.spi.CDI;

/**
 * IO implementation that logs all reads and writes performed during a particular currentRights before forwarding the
 * currentRights onto downstream IO modules.
 * The primary use-case for this IO module is for cache-invalidation.
 */
final public class AccessLogIO implements IAteIO {

    private IAteIO next;
    private final RequestAccessLog logger;

    public AccessLogIO(IAteIO next) {
        this.next = next;
        this.logger = CDI.current().select(RequestAccessLog.class).get();
    }

    @Override
    public @Nullable MessageDataHeaderDto readRootOfTrust(PUUID id) {
        return next.readRootOfTrust(id);
    }

    @Override
    public void warm(IPartitionKey partitionKey) { next.warm(partitionKey); }

    @Override
    public void warmAndWait(IPartitionKey partitionKey) { next.warmAndWait(partitionKey); }

    @Override
    public MessageSyncDto beginSync(IPartitionKey partitionKey, MessageSyncDto sync) {
        return next.beginSync(partitionKey, sync);
    }

    @Override
    public boolean finishSync(IPartitionKey partitionKey, MessageSyncDto sync) { return next.finishSync(partitionKey, sync); }

    @Override
    public List<LostDataDto> getLostMessages(IPartitionKey partitionKey) {
        return next.getLostMessages(partitionKey);
    }

    @Override
    public DataSubscriber backend() {
        return next.backend();
    }

    @Override
    public @Nullable MessagePublicKeyDto publicKeyOrNull(IPartitionKey partitionKey, @Hash String hash) {
        return next.publicKeyOrNull(partitionKey, hash);
    }

    @Override
    public void send(DataTransaction transaction, boolean validate) {
        next.send(transaction, validate);

        for (IPartitionKey key : transaction.allKeys()) {
            Map<UUID, MessageDataDto> datas = transaction.getSavedDataMap(key);
            for (Map.Entry<UUID, MessageDataDto> pair : datas.entrySet()) {
                MessageDataHeaderDto header = pair.getValue().getHeader();
                logger.recordWrote(pair.getKey(), header.getPayloadClazzShortOrThrow());
            }
        }
    }

    @Override
    public boolean exists(@Nullable PUUID _id) {
        @DaoId PUUID id = _id;
        if (id == null) return false;
        return next.exists(id);
    }

    @Override
    public boolean everExisted(@Nullable PUUID _id) {
        @DaoId PUUID id = _id;
        if (id == null) return false;
        return next.everExisted(id);
    }

    @Override
    public boolean immutable(PUUID id) {
        return next.immutable(id);
    }

    @Override
    public @Nullable BaseDao readOrNull(PUUID id) {
        BaseDao ret = next.readOrNull(id);
        if (ret != null) {
            logger.recordRead(id.id(), ret.getClass());
        }
        return ret;
    }

    @Override
    public BaseDao readOrThrow(PUUID id) {
        BaseDao ret = next.readOrThrow(id);
        if (ret != null) {
            logger.recordRead(id.id(), ret.getClass());
        }
        return ret;
    }

    @Override
    public @Nullable DataContainer readRawOrNull(PUUID id) {
        DataContainer ret = next.readRawOrNull(id);
        if (ret != null) {
            logger.recordRead(id.id(), ret.getPayloadClazzShort());
        }
        return ret;
    }

    @Override
    public @Nullable BaseDao readVersionOrNull(PUUID id, long offset) {
        return next.readVersionOrNull(id, offset);
    }

    @Override
    public @Nullable MessageDataMetaDto readVersionMsgOrNull(PUUID id, long offset) {
        return next.readVersionMsgOrNull(id, offset);
    }

    @Override
    public <T extends BaseDao> Iterable<MessageMetaDto> readHistory(PUUID id, Class<T> clazz) {
        Iterable<MessageMetaDto> ret = next.readHistory(id, clazz);
        logger.recordRead(id.id(), clazz);
        return ret;
    }

    @Override
    public List<BaseDao> view(IPartitionKey partitionKey, Predicate<BaseDao> predicate) {
        return next.view(partitionKey, predicate);
    }

    @Override
    public <T extends BaseDao> List<T> view(IPartitionKey partitionKey, Class<T> type, Predicate<T> predicate) {
        List<T> ret = next.view(partitionKey, type, predicate);
        logger.recordRead(type);
        return ret;
    }

    @Override
    public <T extends BaseDao> List<DataContainer> readAllRaw(IPartitionKey partitionKey) {
        return next.readAllRaw(partitionKey);
    }

    @Override
    public <T extends BaseDao> List<DataContainer> readAllRaw(IPartitionKey partitionKey, Class<T> type) {
        List<DataContainer> ret = next.readAllRaw(partitionKey, type);
        logger.recordRead(type);
        return ret;
    }
}
