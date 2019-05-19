package com.tokera.ate;

import com.google.common.base.Stopwatch;
import com.tokera.ate.common.ApplicationConfigLoader;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.common.NetworkTools;
import com.tokera.ate.configuration.AteConstants;
import com.tokera.ate.delegates.AteDelegate;
import org.apache.zookeeper.server.ServerConfig;
import org.apache.zookeeper.server.ZooKeeperServerMain;
import org.apache.zookeeper.server.quorum.QuorumPeerConfig;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.slf4j.LoggerFactory;

import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import javax.ws.rs.WebApplicationException;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.io.PrintWriter;
import java.util.*;
import java.util.concurrent.TimeUnit;

@ApplicationScoped
public class ZooServer implements Runnable {
    
    private static final org.slf4j.Logger SLOG = LoggerFactory.getLogger(ZooServer.class);

    protected AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;
    
    private @MonotonicNonNull Thread thread;
    private volatile boolean isRunning = true;
    private volatile boolean isLoaded = true;
    
    @PostConstruct
    public void init() {
        // Create the thread but dont start it yet
        this.thread = new Thread(this);
        this.thread.setDaemon(true);
    }
    
    public void start(boolean shouldRun)
    {
        List<String> zkServers = new ArrayList<>();

        // Load the list of ZooKeeper servers (bootstrap)
        String bootstrapZooKeeper = BootstrapConfig.propertyOrThrow(d.bootstrapConfig.propertiesForAte(), "zookeeper.bootstrap");
        Integer bootstrapZooKeeperPort = NetworkTools.extractPortFromBootstrapOrThrow(bootstrapZooKeeper);

        // Load the properties
        Properties props = d.bootstrapConfig.propertiesForZooKeeper();

        // Get all my local IP addresses
        Set<String> myAddresses = NetworkTools.getMyNetworkAddresses();

        // Loop through all the data servers and process them
        String myAdvertisingIp = null;
        Integer myId = 0;
        List<String> dataservers = d.implicitSecurity.enquireDomainAddresses(bootstrapZooKeeper, true);
        int n = 0;
        for (String serverIp : dataservers) {
            n++;

            zkServers.add(serverIp + ":2888:3888");

            SLOG.info("ZookeeperBootstrap(" + n + ")->" + serverIp + ":" + bootstrapZooKeeperPort);
            if (myAddresses.contains(serverIp)) {
                myAdvertisingIp = serverIp;
                myId = n;
            }
        }
        shouldRun = myAdvertisingIp != null;
        
        if (shouldRun == false) {
            SLOG.info("ZooKeeper should not run on this server");
            return;
        } else {
            SLOG.info("ZooKeeper required on this node");
        }

        String dataDir = props.getOrDefault("dataDir", "/opt/zookeeper").toString();
        
        if ("1".equals(MapTools.getOrNull(props,"tokera.autogen.servers"))) {
            for (int x = 0; n < zkServers.size(); x++) {
                Integer index = x + 1;
                props.put("server." + index, zkServers.get(n));
            }
        }
        
        try (PrintWriter out = new PrintWriter(dataDir + "/myid")) {
            out.println(myId.toString());
        } catch (FileNotFoundException ex) {
            throw new WebApplicationException("Failed to set the zookeeper server ID", ex);
        }
        
        StringBuilder propsLog = new StringBuilder();
        propsLog.append("zookeeper values:\n");
        for (Map.Entry<Object, Object> entry : props.entrySet()) {
            propsLog.append("        ").append(entry.getKey()).append(" = ").append(entry.getValue()).append("\n");
        }
        SLOG.info(propsLog.toString());
        
        isRunning = true;

        Thread thread = this.thread;
        if (thread != null) {
            thread.start();
        }
        
        try {
            Stopwatch loadTime = Stopwatch.createStarted();
            while (this.isLoaded == false) {
                if (loadTime.elapsed(TimeUnit.SECONDS) > 20L) {
                    throw new WebApplicationException("Busy while loading zookeeper");
                }
                Thread.sleep(50);
            }
        } catch (InterruptedException ex) {
            throw new WebApplicationException("Interrupted while loading zookeeper", ex);
        }
    }
    
    public void stop() {
        isRunning = false;
        try {
            Thread thread = this.thread;
            if (thread != null) {
                thread.join();
            }
        } catch (InterruptedException ex) {
            this.LOG.warn(ex);
        }
    }
    
    @Override
    public void run() {
        Long errorWaitTime = 500L;
        while (isRunning)
        {
            try {

                ZooKeeperServerMain server = new ZooKeeperServerMain();
        
                QuorumPeerConfig quorumConfiguration = new QuorumPeerConfig();

                // Load the properties
                String propsFilename = d.bootstrapConfig.getPropertiesFileKafka();
                Properties props = ApplicationConfigLoader.getInstance().getPropertiesByName(propsFilename);
                if (props == null) {
                    throw new WebApplicationException("Properties file (" + propsFilename + ") for ATE does not exist.");
                }

                // Load the list of ZooKeeper servers (bootstrap)
                String bootstrapZooKeeper = props.getProperty("zookeeper.bootstrap", null);
                if (bootstrapZooKeeper == null) {
                    throw new RuntimeException("Unable to initialize the ZooKeeper server as no bootstrap address was set (add 'zookeeper.bootstrap' to " + propsFilename + ").");
                }
                Integer bootstrapZooKeeperPort = NetworkTools.extractPortFromBootstrap(bootstrapZooKeeper);
                if (bootstrapZooKeeperPort == null) {
                    throw new RuntimeException("Unable to initialize the ZooKeeper server as no bootstrap port was set (update 'zookeeper.bootstrap' to " + propsFilename + " to include a port number).");
                }

                // Load the properties
                propsFilename = d.bootstrapConfig.getPropertiesFileZooKeeper();
                props = ApplicationConfigLoader.getInstance().getPropertiesByName(propsFilename);
                if (props == null) {
                    throw new WebApplicationException("Properties file (" + propsFilename + ") for ZooKeeper server does not exist.");
                }

                List<String> dataservers = d.implicitSecurity.enquireDomainAddresses(bootstrapZooKeeper, true);
                for (int n = 0; n < dataservers.size(); n++) {
                    String svr = dataservers.get(n);
                    props.put("server." + n, svr + ":" + bootstrapZooKeeperPort);
                }

                try {
                    quorumConfiguration.parseProperties(props);
                } catch(IOException | QuorumPeerConfig.ConfigException e) {
                    throw new RuntimeException(e);
                }

                final ServerConfig configuration = new ServerConfig();
                configuration.readFrom(quorumConfiguration);

                this.isLoaded = true;
                server.runFromConfig(configuration);
                
            } catch (Throwable ex) {
                this.LOG.error(ex);
                try {
                    Thread.sleep(errorWaitTime);
                } catch (InterruptedException ex1) {
                    this.LOG.warn(ex1);
                    break;
                }
                errorWaitTime *= 2L;
                if (errorWaitTime > 4000L) {
                    errorWaitTime = 4000L;
                }
            }
        }
    }

    public void touch() { }
}
