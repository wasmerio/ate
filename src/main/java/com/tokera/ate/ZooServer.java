package com.tokera.ate;

import com.google.common.base.Stopwatch;
import com.tokera.ate.common.ApplicationConfigLoader;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.MapTools;
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
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Properties;
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
        this.thread = new Thread(this);
        this.thread.setDaemon(true);
    }
    
    public void start(boolean shouldRun)
    {
        List<String> zkServers = new ArrayList<>();
        
        String argsIp = System.getProperty(AteConstants.PROPERTY_ARGS_IP, null);
        if (argsIp == null) argsIp = "127.0.0.1";
        String argsPort0 = System.getProperty(AteConstants.PROPERTY_ARGS_PORT, null);
        if (argsPort0 == null) argsPort0 = "9092";
        Integer argsPortStride = Integer.parseInt(argsPort0) - 9092;
        Integer argsPort1 = 2181 + argsPortStride;
        
        Integer myId = 0;
        String bootstraps = d.implicitSecurity.enquireDomainString(d.bootstrapConfig.zookeeperAlias + "." + d.bootstrapConfig.domain, true);
        if (bootstraps != null) {
            SLOG.info(d.bootstrapConfig.zookeeperAlias + "."  + d.bootstrapConfig.domain + "->" + bootstraps);
            int n = 0;
            for (String svr : bootstraps.split("\\,")) {
                n++;
                String[] comps = svr.split("\\:");
                if (comps.length < 1) continue;
                
                String serverIp = comps[0];
                String serverPort = "2181";
                if (comps.length >= 2) serverPort = comps[1];
                
                Integer portStride = Integer.parseInt(serverPort) - 2181;
                Integer portInternal1 = 2888 + portStride;
                Integer portInternal2 = 3888 + portStride;
                
                zkServers.add(serverIp + ":" + portInternal1 + ":" + portInternal2);

                SLOG.info("ZookeeperBootstrap(" + n + ")->" + serverIp + ":" + serverPort);
                
                if (serverIp.equalsIgnoreCase(argsIp) &&
                        (serverPort.equalsIgnoreCase(argsPort1.toString()) || serverPort.equalsIgnoreCase("2181")))
                {
                    shouldRun = true;
                    myId = n;
                }
            }
        }
        
        if (shouldRun == false) {
            SLOG.info("ZooKeeper should not run on this server");
            return;
        } else {
            SLOG.info("ZooKeeper required on this node");
        }
        
        Properties props = ApplicationConfigLoader.getInstance().getPropertiesByName(System.getProperty(AteConstants.PROPERTY_ZOOKEEPER_SYSTEM));
        if (props == null) {
            throw new WebApplicationException("Zookeeper configuration file is missing");
        }
        String dataDir = props.getOrDefault("dataDir", "/opt/zookeeper").toString();
        
        if ("1".equals(MapTools.getOrNull(props,"tokera.autogen.servers"))) {
            for (int n = 0; n < zkServers.size(); n++) {
                Integer index = n + 1;
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
                
                Properties props = ApplicationConfigLoader.getInstance().getPropertiesByName(System.getProperty(AteConstants.PROPERTY_ZOOKEEPER_SYSTEM));
                if (props == null) throw new WebApplicationException("Zookeeper configuration file missing");
                
                String bootstraps = d.implicitSecurity.enquireDomainString(d.bootstrapConfig.zookeeperAlias + "." + d.bootstrapConfig.domain, true);
                if (bootstraps != null) {
                    String[] servers = bootstraps.split("\\,");
                    for (int n = 0; n < servers.length; n++) {
                        String svr = servers[n];
                        
                        String[] comps = svr.split("\\:");
                        if (comps.length >= 2) {
                            int port = Integer.parseInt(comps[1]);
                            port += 1000;
                            svr = svr + ":" + port;
                        }

                        props.put("server." + n, svr);
                    }
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
