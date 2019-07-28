package com.tokera.ate.io.repo;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.msg.MessageSync;
import com.tokera.ate.dto.msg.MessageBaseDto;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageMetaDto;
import com.tokera.ate.dto.msg.MessageSyncDto;
import com.tokera.ate.io.api.IPartitionKey;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.ws.rs.WebApplicationException;
import java.util.Set;
import java.util.UUID;

/**
 * Represents an interface that will stream data messages to and from a persistent storage (e.g. Kafka BUS or Local Data File)
 */
public interface IDataTopicBridge {

    void send(IPartitionKey key, MessageBaseDto msg);

    void waitTillLoaded(IPartitionKey key);

    IDataPartitionBridge addKey(IPartitionKey key);

    boolean removeKey(IPartitionKey key);

    Set<IPartitionKey> keys();

    boolean sync(IPartitionKey key);

    MessageSyncDto startSync(IPartitionKey key);

    MessageSyncDto startSync(IPartitionKey key, MessageSyncDto sync);

    boolean finishSync(IPartitionKey key, MessageSyncDto sync);

    boolean finishSync(IPartitionKey key, MessageSyncDto sync, int timeout);

    boolean hasFinishSync(IPartitionKey key, MessageSyncDto sync);

    @Nullable MessageDataDto getVersion(PUUID id, MessageMetaDto meta);
}
