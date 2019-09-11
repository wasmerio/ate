package com.tokera.ate.io.repo;

import com.tokera.ate.dao.MessageBundle;
import com.tokera.ate.dao.msg.MessageSync;
import com.tokera.ate.dto.msg.MessageBaseDto;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageMetaDto;
import com.tokera.ate.dto.msg.MessageSyncDto;
import com.tokera.ate.io.api.IPartitionKey;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.UUID;

/**
 * Represents an interface that will stream data messages to and from a persistent storage (e.g. Kafka BUS or Local Data File)
 */
public interface IDataPartitionBridge {

    void send(MessageBaseDto msg);

    void waitTillLoaded();

    boolean sync();

    MessageSyncDto startSync();

    MessageSyncDto startSync(MessageSyncDto sync);

    boolean finishSync(MessageSyncDto sync);

    boolean finishSync(MessageSyncDto sync, int timeout);

    boolean hasFinishSync(MessageSyncDto sync);

    @Nullable MessageDataDto getVersion(UUID id, MessageMetaDto meta);

    IPartitionKey partitionKey();

    DataPartitionChain chain();

    void feed(Iterable<MessageBundle> msgs);
}
