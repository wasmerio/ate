package com.tokera.ate.io.kafka;

import com.tokera.ate.common.ApplicationConfigLoader;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.GenericPartitionKey;
import com.tokera.ate.dao.kafka.MessageSerializer;
import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageBaseDto;
import com.tokera.ate.dto.msg.MessageSyncDto;
import com.tokera.ate.enumerations.DataPartitionType;
import kafka.admin.AdminUtils;
import kafka.utils.ZKStringSerializer$;
import org.I0Itec.zkclient.ZkClient;
import org.I0Itec.zkclient.ZkConnection;
import org.apache.kafka.clients.producer.KafkaProducer;
import org.apache.kafka.clients.producer.ProducerRecord;
import org.apache.kafka.common.errors.TopicExistsException;

import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import java.util.Properties;
import java.util.concurrent.ConcurrentSkipListSet;

@ApplicationScoped
public class KafkaTopicFactory {
    protected AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;

    private ConcurrentSkipListSet<String> everCreated = new ConcurrentSkipListSet<>();

    public final static int maxTopics = 10000;
    public final static int maxPartitions = 200000;
    public final static int maxPartitionsPerTopic = maxPartitions / maxTopics;

    public enum Response
    {
        WasCreated,
        AlreadyExists,
        Failed
    }

    /**
     * Initializes the partition by creating it
     */
    @SuppressWarnings( "deprecation" )
    public Response create(String topic, DataPartitionType type)
    {
        // If the topic has ever been created by this TokAPI then we dont attempt it again
        if (everCreated.contains(topic)) {
            return Response.AlreadyExists;
        }

        synchronized (this)
        {
            // Load the properties for the zookeeper instance
            Properties props = d.bootstrapConfig.propertiesForKafka();

            // Add the bootstrap to the configuration file
            String zookeeperHosts = d.kafka.getZooKeeperBootstrap();
            props.put("zookeeper.connect", zookeeperHosts);

            int connectionTimeOutInMs = 10000;
            Object connectionTimeOutInMsObj = MapTools.getOrNull(props, "zookeeper.connection.timeout.ms");
            if (connectionTimeOutInMsObj != null) {
                try {
                    connectionTimeOutInMs = Integer.parseInt(connectionTimeOutInMsObj.toString());
                } catch (NumberFormatException ex) {
                }
            }
            int sessionTimeOutInMs = 10000;

            int numOfReplicas = 1;
            Object numOfReplicasObj = MapTools.getOrNull(props, "default.replication.factor");
            if (numOfReplicasObj != null) {
                try {
                    numOfReplicas = Integer.parseInt(numOfReplicasObj.toString());
                } catch (NumberFormatException ex) {
                }
            }

            ZkClient client = new ZkClient(zookeeperHosts, sessionTimeOutInMs, connectionTimeOutInMs, ZKStringSerializer$.MODULE$);
            kafka.utils.ZkUtils utils = new kafka.utils.ZkUtils(client, new ZkConnection(zookeeperHosts), false);

            // If it already exists the nwe are done
            if (AdminUtils.topicExists(utils, topic)) {
                everCreated.add(topic);
                return Response.AlreadyExists;
            }

            // Load the topic properties depending on the need
            String topicPropsName;
            switch (type) {
                default:
                case Dao:
                    topicPropsName = d.bootstrapConfig.getPropertiesFileTopicDao();
                    break;
                case Io:
                    topicPropsName = d.bootstrapConfig.getPropertiesFileTopicIo();
                    break;
                case Publish:
                    topicPropsName = d.bootstrapConfig.getPropertiesFileTopicPublish();
                    break;
            }
            Properties topicProps = ApplicationConfigLoader.getInstance().getPropertiesByName(topicPropsName);

            // Enter a retry loop with exponential backoff
            int delayMs = 100;
            for (int n = 0;; n++)
            {
                if (topicProps != null) {
                    // Create the topic
                    try {
                        AdminUtils.createTopic(utils, topic, maxPartitionsPerTopic, numOfReplicas, topicProps, kafka.admin.RackAwareMode.Disabled$.MODULE$);
                        everCreated.add(topic);
                        return Response.WasCreated;
                    } catch (TopicExistsException ex) {
                        everCreated.add(topic);
                        return Response.AlreadyExists;
                    } catch (Throwable ex) {
                        if (n >= 7) {
                            LOG.warn(ex);
                            return Response.Failed;
                        }
                        try {
                            Thread.sleep(delayMs);
                        } catch (InterruptedException e) {
                            LOG.warn(ex);
                            return Response.Failed;
                        }
                        delayMs *= 2;
                        continue;
                    }
                }
            }
        }
    }
}
