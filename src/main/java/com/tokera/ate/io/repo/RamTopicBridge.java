package com.tokera.ate.io.repo;

import com.tokera.ate.common.MapTools;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.enumerations.DataTopicType;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.spongycastle.crypto.InvalidCipherTextException;

import java.io.IOException;
import java.util.*;

/**
 * Represents a bridge of a particular topic with an in memory RAM copy of the data
 */
public class RamTopicBridge implements IDataTopicBridge {

    private DataTopicChain chain;
    private DataTopicType type;
    private final Random rand = new Random();
    private RamTopicPartition partition;

    public RamTopicBridge(DataTopicChain chain, DataTopicType type, RamTopicPartition p) {
        this.chain = chain;
        this.type = type;
        this.partition = p;

        RamTopicBridge.seed(chain.getTopicName(), chain, p);
    }

    private static void seed(String topicName, DataTopicChain chain, RamTopicPartition p) {
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
    public void start() {

    }

    @Override
    public void stop() {

    }

    @Override
    public boolean ethereal() {
        return false;
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
}