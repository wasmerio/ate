 package com.tokera.ate.io.kafka;

import com.tokera.ate.common.ApplicationConfigLoader;
import com.tokera.ate.configuration.AteConstants;
import com.tokera.ate.dao.msg.MessageBase;
import java.util.Map;
import java.util.Properties;
import java.util.UUID;
import javax.enterprise.context.ApplicationScoped;

import com.tokera.ate.delegates.AteDelegate;
import org.apache.kafka.clients.consumer.ConsumerConfig;
import org.apache.kafka.clients.consumer.KafkaConsumer;
import org.apache.kafka.clients.producer.KafkaProducer;

/**
 * Generates Kafka configuration files for various situations and creates the Kafka producers and consumers
 */
@ApplicationScoped
public class KafkaConfigTools {
    
    public enum TopicRole {
        Consumer,
        Producer
    }
    
    public enum TopicType {
        Dao,
        Io,
        Publish
    }
    
    public KafkaConfigTools() {
    }
    
    public Properties generateConfig(TopicRole role, TopicType type, String bootstraps) {
        AteDelegate d = AteDelegate.get();

        String topicRole;
        switch (role) {
            case Producer:
                topicRole = d.bootstrapConfig.getPropertiesFileProducer();
                break;
            default:
            case Consumer:
                topicRole = d.bootstrapConfig.getPropertiesFileConsumer();
                break;
        }
        
        Properties config = ApplicationConfigLoader.getInstance().getPropertiesByName(topicRole);
        if (config == null) config = new Properties();
        
        if (role == TopicRole.Consumer) {
            config.put(ConsumerConfig.GROUP_ID_CONFIG, UUID.randomUUID().toString());
            config.put(ConsumerConfig.CLIENT_ID_CONFIG, UUID.randomUUID().toString());
        }
        
        String topicType;
        switch (type) {
            default:
            case Dao:
                topicType = d.bootstrapConfig.getPropertiesFileTopicDao();
                break;
            case Io:
                topicType = d.bootstrapConfig.getPropertiesFileTopicIo();
                break;
            case Publish:
                topicType = d.bootstrapConfig.getPropertiesFileTopicPublish();
                break;
        }

        Properties entries = ApplicationConfigLoader.getInstance().getPropertiesByName(topicType);
        if (entries != null) {
            for (Map.Entry<Object, Object> pair : entries.entrySet()){
                config.put(pair.getKey(), pair.getValue());
            }
        }
        
        config.put("bootstrap.servers", bootstraps);
        return config;
    }
    
    public KafkaProducer<String, MessageBase> newProducer(TopicRole role, TopicType type, String bootstraps) {
        return new KafkaProducer<>(generateConfig(role, type, bootstraps));
    }
    
    public KafkaConsumer<String, MessageBase> newConsumer(TopicRole role, TopicType type, String bootstraps) {
        return new KafkaConsumer<>(generateConfig(role, type, bootstraps));
    }
}
