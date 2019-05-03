package com.tokera.ate.io.repo;

import com.tokera.ate.scopes.Startup;
import com.tokera.ate.common.ApplicationConfigLoader;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.configuration.AteConstants;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.enumerations.DataTopicType;
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
    private AteDelegate d = AteDelegate.getUnsafe();
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
        // Load the properties
        @Nullable Properties props = ApplicationConfigLoader.getInstance().getPropertiesByName(System.getProperty(AteConstants.PROPERTY_KAFKA_SYSTEM));
        if (props == null) {
            exceptionOnUse = new RuntimeException("Must have a kafka configuration file to use the KafkaRepository.");
            return;
        }

        String bootstraps = d.implicitSecurity.enquireDomainString(d.bootstrapConfig.getZookeeperAlias() + "." + d.bootstrapConfig.getDomain(), true);
        if (bootstraps != null) {
            props.put("zookeeper.connect", bootstraps);
        }

        String zookeeperConnect = props.getProperty("zookeeper.connect");
        if (zookeeperConnect == null) {
            exceptionOnUse = new RuntimeException("Kafka configuration file must have a zookeeper.connect propery.");
            return;
        }
        m_keeperServers =  zookeeperConnect;

        String bootstrapServers = d.implicitSecurity.enquireDomainString(d.bootstrapConfig.getKafkaAlias() + "." + d.bootstrapConfig.getDomain(), true);
        if (bootstrapServers == null) {
            exceptionOnUse = new RuntimeException("Unable to find Kafka bootstrap servers [dns: " + d.bootstrapConfig.getKafkaAlias() + "." + d.bootstrapConfig.getDomain() + "].");
            return;
        }
        m_bootstrapServers = bootstrapServers;
    }

    public IDataTopicBridge build(DataTopicChain chain, DataTopicType type) {
        touch();
        return new KafkaTopicBridge(chain, d.kafkaConfig, type, m_keeperServers, m_bootstrapServers);
    }

    public void touch()
    {
        if (exceptionOnUse != null) {
            throw exceptionOnUse;
        }
    }
}
