package com.tokera.ate.io.kafka;

import com.tokera.ate.dao.MessageBundle;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.msg.MessageType;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageBaseDto;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageMetaDto;
import com.tokera.ate.dto.msg.MessageSyncDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.DataPartitionChain;
import com.tokera.ate.io.repo.IDataPartitionBridge;
import com.tokera.ate.io.repo.IDataTopicBridge;
import com.tokera.ate.providers.PartitionKeySerializer;
import org.apache.commons.lang3.time.StopWatch;
import org.bouncycastle.crypto.InvalidCipherTextException;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.Response;
import java.io.IOException;
import java.util.UUID;

public class KafkaPartitionBridge implements IDataPartitionBridge {
    public final KafkaTopicBridge bridge;
    public final IPartitionKey key;
    public final DataPartitionChain chain;
    private volatile boolean isLoaded = false;
    private volatile boolean isLoading = false;

    public KafkaPartitionBridge(KafkaTopicBridge bridge, IPartitionKey key, DataPartitionChain chain) {
        this.bridge = bridge;
        this.key = key;
        this.chain = chain;
    }

    @Override
    public void send(MessageBaseDto msg) {
        bridge.send(key, msg);
    }

    @Override
    public void waitTillLoaded() {
        boolean sentSync = false;
        boolean hasCreated = false;
        boolean startedReload = false;

        if (isLoaded == false) {
            StopWatch waitTime = new StopWatch();
            waitTime.start();
            while (isLoaded == false) {
                if (waitTime.getTime() > 5000L) {
                    if (sentSync == false) {
                        bridge.send(key, new MessageSyncDto(0, 0));
                        sentSync = true;
                    }
                }
                if (waitTime.getTime() > 8000L) {
                    if (startedReload == false) {
                        bridge.inbox.reload();
                        startedReload = true;
                    }
                }
                if (waitTime.getTime() > 15000L) {
                    if (hasCreated == false) {
                        createTopic();
                        bridge.inbox.reload();
                        hasCreated = true;
                    }
                }
                if (waitTime.getTime() > 25000L) {
                    throw new RuntimeException("Busy loading data partition [" + PartitionKeySerializer.toString(key) + "]");
                }
                try {
                    Thread.sleep(50);
                } catch (InterruptedException ex) {
                    break;
                }
            }
        }
    }

    public void createTopic()
    {
        // Make sure the topic is actually created
        KafkaTopicFactory.Response response = AteDelegate.get().kafkaTopicFactory.create(key.partitionTopic(), key.partitionType());
        switch (response) {
            case AlreadyExists: {
                break;
            }
            case WasCreated: {
                AteDelegate.get().genericLogger.info("partition [" + this.key + "]: loaded-created");
                isLoaded = true;
                break;
            }
            case Failed: {
                throw new WebApplicationException("Failed to create the new partitions.", Response.Status.INTERNAL_SERVER_ERROR);
            }
        }
    }

    @Override
    public boolean sync() {
        return bridge.sync(key);
    }

    @Override
    public MessageSyncDto startSync(MessageSyncDto sync) {
        return bridge.startSync(key, sync);
    }

    @Override
    public MessageSyncDto startSync() {
        return bridge.startSync(key);
    }

    @Override
    public boolean finishSync(MessageSyncDto sync) {
        return bridge.finishSync(key, sync);
    }

    @Override
    public boolean finishSync(MessageSyncDto sync, int timeout) {
        return bridge.finishSync(key, sync, timeout);
    }

    @Override
    public boolean hasFinishSync(MessageSyncDto sync) {
        return bridge.hasFinishSync(key, sync);
    }

    @Override
    public @Nullable MessageDataDto getVersion(UUID id, MessageMetaDto meta) {
        return bridge.getVersion(PUUID.from(key, id), meta);
    }

    @Override
    public IDataTopicBridge topicBridge() {
        return this.bridge;
    }

    @Override
    public IPartitionKey partitionKey() {
        return this.key;
    }

    @Override
    public DataPartitionChain chain() {
        return this.chain;
    }

    @Override
    public void feed(Iterable<MessageBundle> msgs)
    {
        // Now find the bridge and send the message to it
        for  (MessageBundle bundle : msgs)
        {
            // Now process the message itself
            MessageMetaDto meta = new MessageMetaDto(
                    bundle.partition,
                    bundle.offset);

            if (bundle.raw.msgType() == MessageType.MessageSync) {
                bridge.processSync(new MessageSyncDto(bundle.raw));
                return;
            }
            try {
                chain.rcv(bundle.raw, meta, isLoaded, bridge.LOG);
            } catch (IOException | InvalidCipherTextException ex) {
                bridge.LOG.warn(ex);
            }
        }

        // Set the loading flag
        if (isLoading == false) {
            isLoading = true;
        }
    }

    @Override
    public void idle() {
        if (isLoaded == false && isLoading) {
            isLoaded = true;
            AteDelegate.get().genericLogger.info("partition [" + this.key + "]: loaded");
        }
    }
}
