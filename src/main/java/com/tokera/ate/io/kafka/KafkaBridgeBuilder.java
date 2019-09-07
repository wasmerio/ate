package com.tokera.ate.io.kafka;

import com.tokera.ate.KafkaServer;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.DataPartitionChain;
import com.tokera.ate.io.repo.IDataTopicBridge;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.enumerations.DataPartitionType;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;

/**
 * Bridge between the data tree in memory and the Kafka BUS that persists those messages
 */
@ApplicationScoped
public class KafkaBridgeBuilder {

    private @Nullable RuntimeException exceptionOnUse = null;
    private AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;
    @SuppressWarnings("initialization.fields.uninitialized")
    private String m_bootstrapServers;

    public KafkaBridgeBuilder() {
    }

    @PostConstruct
    public void init()
    {
        try {
            m_bootstrapServers = KafkaServer.getKafkaBootstrap();
        } catch (RuntimeException ex) {
            exceptionOnUse = ex;
        }
    }

    public IDataTopicBridge build(String topic, DataPartitionType type) {
        touch();
        return new KafkaTopicBridge(topic, d.kafkaConfig, type, m_bootstrapServers);
    }

    public void touch()
    {
        if (exceptionOnUse != null) {
            throw exceptionOnUse;
        }
    }
}
