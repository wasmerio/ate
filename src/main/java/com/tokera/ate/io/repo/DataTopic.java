package com.tokera.ate.io.repo;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.extensions.DaoParentDiscoveryExtension;
import com.tokera.ate.delegates.YamlDelegate;
import com.tokera.ate.dto.msg.MessageBaseDto;
import com.tokera.ate.enumerations.DataTopicType;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.ws.rs.core.Response;

/**
 * Represents a Topic of data held within the data repositories that make up the storage backend.
 */
public class DataTopic {
    
    private final DataTopicChain chain;
    private final IDataTopicBridge bridge;
    private final DataTopicType type;
    private final DaoParentDiscoveryExtension parentDiscovery;
    
    public DataTopic(DataTopicChain chain, IDataTopicBridge bridge, DataTopicType type, DaoParentDiscoveryExtension parentDiscovery)
    {
        this.parentDiscovery = parentDiscovery;
        this.chain = chain;
        this.bridge = bridge;
        this.type = type;
    }
    
    public void start() {
        bridge.start();
    }
    
    public void stop() {
        bridge.stop();
    }
    
    public void waitTillLoaded() {
        bridge.waitTillLoaded();
    }

    public DataTopicChain getChain() {
        return chain;
    }

    public IDataTopicBridge getBridge() {
        return bridge;
    }
    
    public boolean ethereal() {
        return bridge.ethereal();
    }

    private void debugWrite(MessageBaseDto msg, @Nullable LoggerHook LOG) {
        boolean hideDebug = false;
        if (hideDebug == false) {

            String logMsg = "";
            if (DataRepoConfig.g_EnableVerboseLog == true) {
                String fullStackTrace = org.apache.commons.lang.exception.ExceptionUtils.getFullStackTrace(new Throwable());
                logMsg += fullStackTrace + "\n";
            }

            logMsg += "write: [->" + chain.getTopicName() + "]\n" + YamlDelegate.getInstance().serializeObj(msg);

            if (LOG != null) {
                LOG.info(logMsg);
            } else {
                new LoggerHook(DataTopic.class).info(logMsg);
            }
        }
    }
    
    public void write(MessageBaseDto msg, @Nullable LoggerHook LOG)
    {        
        // Write some debug information
        if (DataRepoConfig.g_EnableLogging == true ||
            DataRepoConfig.g_EnableLoggingWrite == true) {
            debugWrite(msg, LOG);
        }
        
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
}
