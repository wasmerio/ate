package com.tokera.ate.io.repo;

import com.tokera.ate.common.ApplicationConfigLoader;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.configuration.AteConstants;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.enumerations.DataTopicType;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import javax.faces.bean.ManagedBean;
import javax.inject.Inject;
import javax.ws.rs.WebApplicationException;
import java.util.*;

/**
 * Bridge between the data tree in memory and the Kafka BUS that persists those messages
 */
@ApplicationScoped
public class KafkaBridgeBuilder {

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
            throw new WebApplicationException("Must have a kafka configuration file to use the KafkaRepository.");
        }

        String bootstraps = d.implicitSecurity.enquireDomainString("tokkeep.tokera.com", true);
        if (bootstraps != null) {
            props.put("zookeeper.connect", bootstraps);
        }

        String zookeeperConnect = props.getProperty("zookeeper.connect");
        if (zookeeperConnect == null) {
            throw new WebApplicationException("Kafka configuration file must have a zookeeper.connect propery.");
        }
        m_keeperServers =  zookeeperConnect;

        String bootstrapServers = d.implicitSecurity.enquireDomainString("tokdata.tokera.com", true);
        if (bootstrapServers == null) {
            throw new WebApplicationException("Unable to find Kafka bootstrap servers [dns: tokdata.tokera.com].");
        }
        m_bootstrapServers = bootstrapServers;
    }

    public IDataTopicBridge build(DataTopicChain chain, DataTopicType type) {
        return new KafkaTopicBridge(chain, d.kafkaConfig, type, m_keeperServers, m_bootstrapServers);
    }

    public void touch() { }
}
