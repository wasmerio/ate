package com.tokera.ate.io.kafka;

import com.tokera.ate.KafkaServer;
import com.tokera.ate.dao.TopicAndPartition;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.DataPartitionChain;
import com.tokera.ate.io.repo.IDataPartitionBridge;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.delegates.AteDelegate;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import javax.ws.rs.WebApplicationException;

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

    public void touch()
    {
        if (exceptionOnUse != null) {
            throw exceptionOnUse;
        }
    }

    public IDataPartitionBridge createPartition(IPartitionKey key) {
        if (key.partitionIndex() >= KafkaTopicFactory.maxPartitionsPerTopic) {
            throw new WebApplicationException("Partition index can not exceed the maximum of " + KafkaTopicFactory.maxPartitionsPerTopic + " per topic.");
        }

        // Create the partition bridge if it does not exist
        KafkaPartitionBridge ret = new KafkaPartitionBridge(d, key, new DataPartitionChain(key));

        // Create the topic if it doesnt exist
        ret.createTopic();
        d.kafkaInbox.addPartition(new TopicAndPartition(ret.where));
        d.dataMaintenance.addPartition(new TopicAndPartition(ret.where));

        ret.sendLoadSync();
        return ret;
    }

    public void removePartition(IPartitionKey key) {
        d.kafkaInbox.removePartition(new TopicAndPartition(key));
        d.dataMaintenance.removePartition(new TopicAndPartition(key));
    }
}
