package com.tokera.ate;

import com.google.common.base.Stopwatch;
import com.tokera.ate.common.ApplicationConfigLoader;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.common.NetworkTools;
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

import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.WebApplicationException;
import java.util.*;
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

    private static String getGenericBootstrap(String propName) {
        AteDelegate d = AteDelegate.get();

        // Load the list of servers (bootstrap)
        String bootstrap = BootstrapConfig.propertyOrThrow(d.bootstrapConfig.propertiesForAte(), propName);
        Integer bootstrapPort = NetworkTools.extractPortFromBootstrapOrThrow(bootstrap);

        List<String> bootstrapServers = d.implicitSecurity.enquireDomainAddresses(bootstrap, true);
        if (bootstrapServers == null) {
            throw new RuntimeException("Failed to find the " + propName + " list at " + bootstrap);
        }

        // Build a list of all the servers we will connect to
        StringBuilder sb = new StringBuilder();
        if (bootstrapServers != null) {

            for (String bootstrapServer : bootstrapServers) {
                if (sb.length() > 0) sb.append(",");
                sb.append(bootstrapServer).append(":").append(bootstrapPort);
            }
        }
        return sb.toString();

    }

    public static String getZooKeeperBootstrap() {
        return getGenericBootstrap("zookeeper.bootstrap");
    }

    public static String getKafkaBootstrap() {
        return getGenericBootstrap("kafka.bootstrap");
    }

    public void init() {

        // Load the list of Kafka servers (bootstrap)
        String bootstrapKafka = BootstrapConfig.propertyOrThrow(d.bootstrapConfig.propertiesForAte(), "kafka.bootstrap");
        Integer bootstrapKafkaPort = NetworkTools.extractPortFromBootstrapOrThrow(bootstrapKafka);

        // Load the properties
        Properties props = d.bootstrapConfig.propertiesForKafka(SLOG);

        // Get all my local IP addresses
        Set<String> myAddresses = NetworkTools.getMyNetworkAddresses();

        // Loop through all the data servers and process them
        String myAdvertisingIp = null;
        Integer myId = 0;
        List<String> dataservers = d.implicitSecurity.enquireDomainAddresses(bootstrapKafka, true);
        int n = 0;
        for (String serverIp : dataservers) {
            n++;

            SLOG.info("KafkaBootstrap(" + n + ")->" + serverIp + ":" + bootstrapKafkaPort);
            if (myAddresses.contains(serverIp)) {
                myAdvertisingIp = serverIp;
                myId = n;
            }
        }
        shouldRun = myAdvertisingIp != null;

        // Add the bootstrap
        props.put("zookeeper.connect", KafkaServer.getZooKeeperBootstrap());

        // Fix the advertised ports and IPs if we are on a real public IP
        if (myAdvertisingIp != null) {
            props.put("advertised.host.name", myAdvertisingIp);
            props.put("advertised.listeners", "PLAINTEXT://" + myAdvertisingIp + ":" + bootstrapKafkaPort);
        }
        props.put("advertised.port", bootstrapKafkaPort);

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
    
    public KafkaServer start()
    {
        init();

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

        return this;
    }
    
    public void stop() {
        getKafkaServer().shutdown();
    }

    public void restart() {
        stop();
        start();
    }

    public void touch() { }
}