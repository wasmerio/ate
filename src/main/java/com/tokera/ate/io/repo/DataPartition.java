package com.tokera.ate.io.repo;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.dao.MessageBundle;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.extensions.DaoParentDiscoveryExtension;
import com.tokera.ate.dto.msg.MessageBaseDto;
import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.io.api.IPartitionKey;
import org.checkerframework.checker.nullness.qual.Nullable;

/**
 * Represents a partition of data held within the data repositories that make up the storage backend.
 */
public class DataPartition {
    private final AteDelegate d = AteDelegate.get();

    private final IPartitionKey key;
    private final DataPartitionChain chain;
    private final IDataPartitionBridge bridge;
    private final DaoParentDiscoveryExtension parentDiscovery;
    
    public DataPartition(IPartitionKey key, IDataPartitionBridge bridge, DaoParentDiscoveryExtension parentDiscovery)
    {
        this.key = key;
        this.parentDiscovery = parentDiscovery;
        this.chain = bridge.chain();
        this.bridge = bridge;
    }
    
    public void waitTillLoaded() {
        bridge.waitTillLoaded();
    }

    public IPartitionKey partitionKey() { return this.key; }

    public DataPartitionChain getChain() {
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
        bridge.feed(msgs);
    }

    public void idle() {
        bridge.idle();
    }
}
