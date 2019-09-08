package com.tokera.ate.io.kafka;

import com.tokera.ate.KafkaServer;
import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.scopes.Startup;
import org.apache.kafka.clients.consumer.KafkaConsumer;
import org.apache.kafka.clients.producer.KafkaProducer;

import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;

@Startup
@ApplicationScoped
public class KafkaOutbox {
    private AteDelegate d = AteDelegate.get();
    private AtomicReference<KafkaProducer<String, MessageBase>> producer = new AtomicReference<>();

    public KafkaProducer<String, MessageBase> get() {
        for (;;) {
            KafkaProducer<String, MessageBase> ret = this.producer.get();
            if (ret != null) return ret;

            synchronized (this) {
                ret = this.producer.get();
                if (ret != null) return ret;

                ret = d.kafkaConfig.newProducer(KafkaConfigTools.TopicRole.Producer, KafkaConfigTools.TopicType.Dao, KafkaServer.getKafkaBootstrap());
                if (this.producer.compareAndSet(null, ret) == true) {
                    return ret;
                } else {
                    ret.close();
                }
            }
        }
    }
}
