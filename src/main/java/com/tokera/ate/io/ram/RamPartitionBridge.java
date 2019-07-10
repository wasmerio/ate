package com.tokera.ate.io.ram;

import com.tokera.ate.common.MapTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageBaseDto;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageMetaDto;
import com.tokera.ate.dto.msg.MessageSyncDto;
import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.DataPartitionChain;
import com.tokera.ate.io.repo.IDataPartitionBridge;
import com.tokera.ate.io.repo.IDataTopicBridge;
import org.bouncycastle.crypto.InvalidCipherTextException;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.io.IOException;
import java.util.Date;
import java.util.Map;
import java.util.Random;
import java.util.UUID;

/**
 * Represents a bridge of a particular partition with an in memory RAM copy of the data
 */
public class RamPartitionBridge implements IDataPartitionBridge {

    private final RamTopicBridge topicBridge;
    private final DataPartitionChain chain;
    private final DataPartitionType type;
    private final Random rand = new Random();
    private final RamTopicPartition partition;

    public RamPartitionBridge(RamTopicBridge topicBridge, DataPartitionChain chain, DataPartitionType type, RamTopicPartition p) {
        this.topicBridge = topicBridge;
        this.chain = chain;
        this.type = type;
        this.partition = p;

        RamPartitionBridge.seed(chain, p);
    }

    private static void seed(DataPartitionChain chain, RamTopicPartition p) {
        AteDelegate.get().debugLogging.seedingPartitionStart(p.partitionKey, null);

        for (Map.Entry<Long, MessageBaseDto> pair : p.messages.entrySet()) {
            long offset = pair.getKey();
            MessageBaseDto msg = pair.getValue();

            Long timestamp = MapTools.getOrNull(p.timestamps, offset);
            if (timestamp == null) timestamp = 0L;

            try {
                chain.rcv(msg, new MessageMetaDto(p.number, offset, timestamp), p.LOG);
            } catch (IOException | InvalidCipherTextException e) {
                p.LOG.warn(e);
            }
        }
        AteDelegate.get().debugLogging.seedingPartitionEnd(p.partitionKey, null);
    }

    @Override
    public void send(MessageBaseDto msg) {
        long offset = partition.offsetSeed.incrementAndGet();
        long timestamp = new Date().getTime();

        partition.messages.put(offset, msg);
        partition.timestamps.put(offset, timestamp);
        try {
            this.chain.rcv(msg, new MessageMetaDto(partition.number, offset, timestamp), partition.LOG);
        } catch (IOException | InvalidCipherTextException e) {
            partition.LOG.warn(e);
        }
    }

    @Override
    public void waitTillLoaded() {
    }

    @Override
    public boolean sync() {
        return true;
    }

    @Override
    public MessageSyncDto startSync() {
        MessageSyncDto sync = new MessageSyncDto(
                rand.nextLong(),
                rand.nextLong());
        return sync;
    }

    @Override
    public boolean finishSync(MessageSyncDto sync) {
        return true;
    }

    @Override
    public boolean finishSync(MessageSyncDto sync, int timeout) {
        return true;
    }

    @Override
    public boolean hasFinishSync(MessageSyncDto sync) {
        return true;
    }

    @Override
    public @Nullable MessageDataDto getVersion(UUID id, MessageMetaDto meta) {
        long offset = meta.getOffset();
        MessageBaseDto msg = MapTools.getOrNull(partition.messages, offset);
        if (msg == null) return null;

        if (msg instanceof MessageDataDto) {
            return (MessageDataDto)msg;
        }

        return null;
    }

    @Override
    public IDataTopicBridge topicBridge() {
        return this.topicBridge;
    }

    @Override
    public IPartitionKey partitionKey() {
        return this.chain.partitionKey();
    }

    @Override
    public DataPartitionChain chain() {
        return this.chain;
    }
}