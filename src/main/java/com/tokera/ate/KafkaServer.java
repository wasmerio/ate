package com.tokera.ate;

import com.google.common.base.Stopwatch;
import com.tokera.ate.common.ApplicationConfigLoader;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.configuration.AteConstants;
import com.tokera.ate.delegates.AteDelegate;
import kafka.metrics.KafkaMetricsReporter;
import kafka.metrics.KafkaMetricsReporter$;
import kafka.server.KafkaConfig;
import kafka.utils.VerifiableProperties;
import org.apache.kafka.common.utils.SystemTime;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.slf4j.LoggerFactory;
import scala.Option;
import scala.collection.Seq;

import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.WebApplicationException;
import java.util.Properties;
import java.util.concurrent.TimeUnit;

@ApplicationScoped
public class KafkaServer {

    private static final org.slf4j.Logger SLOG = LoggerFactory.getLogger(KafkaServer.class);

    protected AteDelegate d = AteDelegate.get();
    private @MonotonicNonNull KafkaConfig config;
    private @MonotonicNonNull Seq<KafkaMetricsReporter> reporters;
    @SuppressWarnings("initialization.fields.uninitialized")
    private kafka.server.KafkaServer kafkaServer;
    private boolean shouldRun = true;

    @PostConstruct
    public void init() {
        // Load the properties
        Properties props = ApplicationConfigLoader.getInstance().getPropertiesByName(System.getProperty(AteConstants.PROPERTY_KAFKA_SYSTEM));
        if (props == null) {
            throw new WebApplicationException("Properties file for Kafka System does not exist.");
        }

        String argsIp = System.getProperty(AteConstants.PROPERTY_ARGS_IP, null);
        String argsPort = System.getProperty(AteConstants.PROPERTY_ARGS_PORT, null);
        if (argsPort == null) argsPort = "9092";

        boolean detectShouldRun = false;

        Integer numBrokers = 0;
        Integer myId = 0;
        String dataservers = d.implicitSecurity.enquireDomainString(d.bootstrapConfig.getKafkaAlias() + "." + d.bootstrapConfig.getDomain(), true);
        if (dataservers != null) {
            SLOG.info(d.bootstrapConfig.getKafkaAlias() + "." + d.bootstrapConfig.getDomain() + "->" + dataservers);
            int n = 0;
            for (String svr : dataservers.split("\\,")) {
                numBrokers++;
                n++;
                String[] comps = svr.split("\\:");
                if (comps.length < 1) continue;

                String serverIp = comps[0];
                String serverPort = "9092";
                if (comps.length >= 2) serverPort = comps[1];

                SLOG.info("KafkaBootstrap(" + n + ")->" + serverIp + ":" + serverPort);

                if (serverIp.equalsIgnoreCase(argsIp)) {
                    if (serverPort.equalsIgnoreCase("9092")) {
                        System.setProperty(AteConstants.PROPERTY_ARGS_PORT, argsPort);
                        argsPort = "9092";
                    }

                    if (serverPort.equalsIgnoreCase(argsPort)) {
                        detectShouldRun = true;
                        myId = n;
                    }
                }
            }
        }
        shouldRun = detectShouldRun;

        String bootstraps = d.implicitSecurity.enquireDomainString(d.bootstrapConfig.getZookeeperAlias() + "." + d.bootstrapConfig.getDomain(), true);
        if (bootstraps != null) {
            SLOG.info(d.bootstrapConfig.getZookeeperAlias() + "." + d.bootstrapConfig.getDomain() + "->" + bootstraps);
            props.put("zookeeper.connect", bootstraps);
        }

        // Fix the advertised ports and IPs if we are on a real public IP
        if (argsIp != null) {
            props.put("advertised.host.name", argsIp);
            props.put("advertised.listeners", "PLAINTEXT://" + argsIp + ":" + argsPort);
        }
        props.put("advertised.port", argsPort);

        // Cap the number of replicas so they do not exceed the number of brokers
        Integer numOfReplicas = 2;
        Object numOfReplicasObj = MapTools.getOrNull(props, "default.replication.factor");
        if (numOfReplicasObj != null) {
            try {
                numOfReplicas = Integer.parseInt(numOfReplicasObj.toString());
            } catch (NumberFormatException ex) {
            }
        }
        if (numBrokers < 1) numBrokers = 1;
        if (numOfReplicas > numBrokers) numOfReplicas = numBrokers;

        props.put("default.replication.factor", numOfReplicas.toString());
        props.put("transaction.state.log.replication.factor", numOfReplicas.toString());
        SLOG.info("Kafka Replication Factor: " + numOfReplicas);

        if (shouldRun == false) {
            SLOG.info("Kafka Broker should not run on this server");
            return;
        } else {
            SLOG.info("Kafka Broker required on this node");
        }

        props.put("broker.id", myId.toString());
        SLOG.info("This Kafka broker.id=" + myId);

        // Retain the brokerConfig.
        config = new KafkaConfig(props);

        SLOG.info("kafkaConfig: autoCreateTopicsEnable - " + config.autoCreateTopicsEnable());

        // Create the reporters
        reporters = KafkaMetricsReporter$.MODULE$.startReporters(new VerifiableProperties(props));
    }

    @SuppressWarnings({"known.nonnull", "argument.type.incompatible"})
    private kafka.server.KafkaServer getKafkaServer() {
        kafka.server.KafkaServer ret = this.kafkaServer;
        if (ret == null) {
            ret = new kafka.server.KafkaServer(config, new SystemTime(), Option.apply("prefix"), reporters);
            this.kafkaServer = ret;
        }
        return ret;
    }

    @SuppressWarnings("assignment.type.incompatible")
    private void clearKafkaServer() {
        this.kafkaServer = null;
    }
    
    public void start()
    {
        // Enter a processing loop
        Stopwatch loadTime = Stopwatch.createStarted();
        while (true)
        {
            try
            {
                // If we do not need to run then we are finished
                if (shouldRun == false) {
                    break;
                }
                
                // Start the kafka server
                this.getKafkaServer().startup();

            } catch (Throwable ex) {
                // Check for timeout
                if (loadTime.elapsed(TimeUnit.SECONDS) > 20L) {
                    SLOG.error("Busy while loading kafka - exiting");
                    try { kafka.utils.Exit.exit(1, Option.apply("prefix")); } catch (Throwable e) { };
                    System.exit(1);
                }

                this.clearKafkaServer();

                System.gc();
                System.runFinalization();
                try {
                    Thread.sleep(1000);
                } catch (InterruptedException ex1) {
                    SLOG.error("Interrupted while loading kafka", ex);
                    try { kafka.utils.Exit.exit(1, Option.apply("prefix")); } catch (Throwable e) { };
                    System.exit(1);
                }
                System.gc();
                System.runFinalization();
                continue;
            }
            
            // We are finished
            break;
        }
    }
    
    public void stop() {
        getKafkaServer().shutdown();
    }

    public void touch() { }
}