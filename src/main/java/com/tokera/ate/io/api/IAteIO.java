package com.tokera.ate.io.api;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.io.repo.DataContainer;
import com.tokera.ate.io.repo.DataSubscriber;
import com.tokera.ate.io.repo.DataTransaction;
import com.tokera.ate.units.Hash;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.List;

/**
 * Interface used for generic input output operations on data entities
 */
public interface IAteIO {

    boolean exists(@Nullable PUUID id);
    
    boolean everExisted(@Nullable PUUID id);
    
    boolean immutable(PUUID id);

    @Nullable MessageDataHeaderDto readRootOfTrust(PUUID id);

    @Nullable BaseDao readOrNull(PUUID id, boolean shouldSave);

    BaseDao readOrThrow(PUUID id);

    @Nullable DataContainer readRawOrNull(PUUID id);
    
    <T extends BaseDao> Iterable<MessageMetaDto> readHistory(PUUID id, Class<T> clazz);
    
    @Nullable BaseDao readVersionOrNull(PUUID id, MessageMetaDto meta);
    
    @Nullable MessageDataDto readVersionMsgOrNull(PUUID id, MessageMetaDto meta);

    List<BaseDao> readAll(IPartitionKey partitionKey);
    
    <T extends BaseDao> List<T> readAll(IPartitionKey partitionKey, Class<T> type);

    <T extends BaseDao> List<DataContainer> readAllRaw(IPartitionKey partitionKey);

    <T extends BaseDao> List<DataContainer> readAllRaw(IPartitionKey partitionKey, Class<T> type);

    @Nullable MessagePublicKeyDto publicKeyOrNull(IPartitionKey partitionKey, @Hash String hash);

    void send(DataTransaction transaction, boolean validate);

    void warm(IPartitionKey partitionKey);

    void warmAndWait(IPartitionKey partitionKey);

    DataSubscriber backend();

    MessageSyncDto beginSync(IPartitionKey partitionKey, MessageSyncDto sync);

    boolean finishSync(IPartitionKey partitionKey, MessageSyncDto sync);
}
