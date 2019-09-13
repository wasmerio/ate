package com.tokera.ate.io.repo;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.dao.MessageBundle;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageMetaDto;
import com.tokera.ate.dto.msg.MessageSyncDto;
import com.tokera.ate.extensions.DaoParentDiscoveryExtension;
import com.tokera.ate.dto.msg.MessageBaseDto;
import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.io.api.IPartitionKey;
import org.bouncycastle.crypto.InvalidCipherTextException;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.io.IOException;

/**
 * Represents a partition of data held within the data repositories that make up the storage backend.
 */
public class DataPartition {
    private final AteDelegate d = AteDelegate.get();

    private final IPartitionKey key;
    private final DataPartitionChain chain;
    private final IDataPartitionBridge bridge;
    
    public DataPartition(IPartitionKey key, IDataPartitionBridge bridge)
    {
        this.key = key;
        this.chain = bridge.chain();
        this.bridge = bridge;
    }
    
    public void waitTillLoaded() {
        bridge.waitTillLoaded();
    }

    public IPartitionKey partitionKey() { return this.key; }

    public DataPartitionChain getChain(boolean waitForLoad) {
        if (waitForLoad) {
            bridge.waitTillLoaded();
        }
        return chain;
    }

    public IDataPartitionBridge getBridge() {
        return bridge;
    }
    
    public void write(MessageBaseDto msg, @Nullable LoggerHook LOG)
    {        
        // First we validate that the entry is going to be accepted
        try {
            if (chain.validate(msg, LOG) == false) {
                String what = msg.toString();
                throw new RuntimeException("The newly created object was not accepted into the chain of trust [" + what + "]");
            }
        } catch (Throwable ex) {
            throw new RuntimeException("Failed during save operation: " + ex.getMessage(), ex);
        }

        // Send the message off to kafka
        bridge.send(msg);

        // Now add it into the chain of trust
        // TODO Remove this later - we now use transactions on the token scope to ensure data is loaded before reading
        //      instead of doing this. Further it doesnt work across multiple TokAPI instances anyway so its value
        //      was limited from the beginning and created false confidence
        //chain.addTrust(msg, null, LOG);
    }

    public void feed(Iterable<MessageBundle> msgs)
    {
        // Now find the bridge and send the message to it
        for  (MessageBundle bundle : msgs)
        {
            // Now process the message itself
            MessageMetaDto meta = new MessageMetaDto(
                    bundle.partition,
                    bundle.offset);

            MessageBaseDto msg = MessageBaseDto.from(bundle.raw);
            if (msg == null) continue;
            d.debugLogging.logReceive(meta, msg);

            if (msg instanceof MessageSyncDto) {
                d.partitionSyncManager.processSync((MessageSyncDto)msg);
                return;
            }
            try {
                boolean isLoaded = this.bridge.hasLoaded();
                chain.rcv(msg, meta, isLoaded, d.genericLogger);
            } catch (IOException | InvalidCipherTextException ex) {
                d.genericLogger.warn(ex);
            }
        }
    }
}
