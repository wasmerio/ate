package com.tokera.ate.io.kafka;

import com.tokera.ate.KafkaServer;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.DataPartitionChain;
import com.tokera.ate.io.repo.IDataPartitionBridge;
import com.tokera.ate.scopes.Startup;
import com.tokera.ate.common.ApplicationConfigLoader;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.configuration.AteConstants;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.enumerations.DataPartitionType;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import java.util.*;

/**
 * Bridge between the data tree in memory and the Kafka BUS that persists those messages
 */
@Startup
@ApplicationScoped
public class KafkaBridgeBuilder {

    private @Nullable RuntimeException exceptionOnUse = null;
    private AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;
    @SuppressWarnings("initialization.fields.uninitialized")
    private String m_bootstrapServers;
    @SuppressWarnings("initialization.fields.uninitialized")
    private String m_keeperServers;

    public KafkaBridgeBuilder() {
    }

    @PostConstruct
    public void init()
    {
        try {
            m_keeperServers = KafkaServer.getZooKeeperBootstrap();
            m_bootstrapServers = KafkaServer.getKafkaBootstrap();
        } catch (RuntimeException ex) {
            exceptionOnUse = ex;
        }
    }

    public IDataPartitionBridge build(IPartitionKey key, DataPartitionChain chain, DataPartitionType type) {
        touch();
        return new KafkaPartitionBridge(key, chain, d.kafkaConfig, type, m_bootstrapServers);
    }

    public void touch()
    {
        if (exceptionOnUse != null) {
            throw exceptionOnUse;
        }
    }
}
