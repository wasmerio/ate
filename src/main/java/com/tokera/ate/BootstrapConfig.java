package com.tokera.ate;

import com.tokera.ate.common.ApplicationConfigLoader;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.enumerations.KeyType;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.enumerations.DefaultStorageSystem;
import com.tokera.ate.enumerations.EnquireDomainKeyHandling;
import com.tokera.ate.scopes.Startup;

import javax.enterprise.context.ApplicationScoped;
import java.util.ArrayList;
import java.util.List;
import java.util.Properties;
import java.util.concurrent.ConcurrentHashMap;

@Startup
@ApplicationScoped
public class BootstrapConfig {
    private Class<?> applicationClass = BootstrapApp.class;
    private String restApiPath = "/rs";
    private String deploymentName = "MyAPI";
    private String pingCheckUrl = "login/ping";
    private String implicitAuthorityAlias = "auth";
    private boolean pingCheckOnStart = false;
    private Integer restPortOverride = null;

    private String dnsServer = "8.8.8.8";

    private List<String> arguments = new ArrayList<>();

    private String propertiesFileAte = "ate.properties";
    private String propertiesFileLog4j = "log4j.properties";
    private String propertiesFileKafka = "kafka.properties";
    private String propertiesFileToken = "token.properties";
    private String propertiesFileZooKeeper = "zookeeper.properties";
    private String propertiesFileConsumer = "consumer.properties";
    private String propertiesFileProducer = "producer.properties";
    private String propertiesFileTopicDao = "topic.dao.properties";
    private String propertiesFileTopicIo = "topic.io.properties";
    private String propertiesFileTopicPublic = "topic.publish.properties";

    private SecurityLevel securityLevel = SecurityLevel.HighlySecure;
    private DefaultStorageSystem defaultStorageSystem = DefaultStorageSystem.Kafka;

    private boolean loggingChainOfTrust = false;
    private boolean loggingMessages = false;
    private boolean loggingMessageData = false;
    private boolean loggingData = false;
    private boolean loggingSync = false;
    private boolean loggingWrites = false;
    private boolean loggingReads = false;
    private boolean loggingDeletes = false;
    private boolean loggingKafka = false;
    private boolean loggingWithStackTrace = false;
    private boolean loggingValidationVerbose = false;
    private boolean loggingCallbacks = false;
    private boolean loggingCallbackData = false;
    private boolean loggingMessageDrops = false;
    private boolean loggingIoStackTraces = false;

    private boolean extraValidation = false;

    private String bootstrapOverrideZookeeper = null;
    private String bootstrapOverrideKafka = null;
    private String kafkaLogDirOverride = null;
    private String zookeeperDataDirOverride = null;

    private int subscriberPartitionTimeout = 120000;
    private int subscriberMaxPartitions = 5000;
    private int dataMaintenanceWindow = 30000;
    private int permissionsCacheLimit = 100000;
    private int indexingMaximumViewsPerTable = 500;
    private int indexingExpireDelay = 120000;
    private boolean enableAutomaticIndexing = true;

    private ConcurrentHashMap<String, Properties> propertiesCache = new ConcurrentHashMap<>();

    // Only weld should initialize this configuration using the ApiServer.startWeld method
    @Deprecated()
    public BootstrapConfig() {
    }

    public Class<?> getApplicationClass() {
        return applicationClass;
    }

    public void setApplicationClass(Class<?> applicationClass) {
        this.applicationClass = applicationClass;
    }

    public String getRestApiPath() {
        return restApiPath;
    }

    public void setRestApiPath(String restApiPath) {
        this.restApiPath = restApiPath;
    }

    public String getDeploymentName() {
        return deploymentName;
    }

    public void setDeploymentName(String deploymentName) {
        this.deploymentName = deploymentName;
    }

    public String getPingCheckUrl() {
        return pingCheckUrl;
    }

    public void setPingCheckUrl(String pingCheckUrl) {
        this.pingCheckUrl = pingCheckUrl;
    }

    public boolean isPingCheckOnStart() {
        return pingCheckOnStart;
    }

    public void setPingCheckOnStart(boolean pingCheckOnStart) {
        this.pingCheckOnStart = pingCheckOnStart;
    }

    public String getPropertiesFileAte() {
        return propertiesFileAte;
    }

    public void setPropertiesFileAte(String propertiesFileAte) {
        this.propertiesFileAte = propertiesFileAte;
    }

    public String getPropertiesFileLog4j() {
        return propertiesFileLog4j;
    }

    public void setPropertiesFileLog4j(String propertiesFileLog4j) {
        this.propertiesFileLog4j = propertiesFileLog4j;
    }

    public String getPropertiesFileKafka() {
        return propertiesFileKafka;
    }

    public void setPropertiesFileKafka(String propertiesFileKafka) {
        this.propertiesFileKafka = propertiesFileKafka;
    }

    public String getPropertiesFileZooKeeper() {
        return propertiesFileZooKeeper;
    }

    public void setPropertiesFileZooKeeper(String propertiesFileZooKeeper) {
        this.propertiesFileZooKeeper = propertiesFileZooKeeper;
    }

    public String getPropertiesFileConsumer() {
        return propertiesFileConsumer;
    }

    public void setPropertiesFileConsumer(String propertiesFileConsumer) {
        this.propertiesFileConsumer = propertiesFileConsumer;
    }

    public String getPropertiesFileProducer() {
        return propertiesFileProducer;
    }

    public void setPropertiesFileProducer(String propertiesFileProducer) {
        this.propertiesFileProducer = propertiesFileProducer;
    }

    public String getPropertiesFileTopicDao() {
        return propertiesFileTopicDao;
    }

    public void setPropertiesFileTopicDao(String propertiesFileTopicDao) {
        this.propertiesFileTopicDao = propertiesFileTopicDao;
    }

    public String getPropertiesFileTopicIo() {
        return propertiesFileTopicIo;
    }

    public void setPropertiesFileTopicIo(String propertiesFileTopicIo) {
        this.propertiesFileTopicIo = propertiesFileTopicIo;
    }

    public String getPropertiesFileTopicPublish() {
        return propertiesFileTopicPublic;
    }

    public void setPropertiesFileTopicPublic(String propertiesFileTopicPublic) {
        this.propertiesFileTopicPublic = propertiesFileTopicPublic;
    }

    public List<String> getArguments() {
        return arguments;
    }

    public void setArguments(List<String> arguments) {
        this.arguments = arguments;
    }

    private Properties getPropertiesFile(String filename, String logicalName) {
        return propertiesCache.computeIfAbsent(filename, f ->
        {
            Properties props = ApplicationConfigLoader.getInstance().getPropertiesByName(filename);
            if (props == null) {
                throw new RuntimeException("Properties file (" + filename + ") for " + logicalName + " does not exist.");
            }
            return props;
        });
    }

    public Properties propertiesForAte() {
        return getPropertiesFile(this.getPropertiesFileAte(), "ATE");
    }

    public Properties propertiesForKafka() {
        return propertiesForKafka(null);
    }

    public Properties propertiesForKafka(org.slf4j.Logger LOG) {
        Properties props = getPropertiesFile(this.getPropertiesFileKafka(), "Kafka");

        String bootstrapKafka = BootstrapConfig.propertyOrThrow(propertiesForAte(), "kafka.bootstrap");
        int numBrokers = AteDelegate.get().implicitSecurity.enquireDomainAddresses(bootstrapKafka, EnquireDomainKeyHandling.ThrowOnError).size();

        // Cap the number of replicas so they do not exceed the number of brokers
        Integer numOfReplicas = 1;
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

        AteDelegate d = AteDelegate.get();
        if (d.bootstrapConfig.getKafkaLogDirOverride() != null) {
            props.put("log.dir", d.bootstrapConfig.getKafkaLogDirOverride());
            props.put("log.dirs", d.bootstrapConfig.getKafkaLogDirOverride());
        }

        if (LOG != null) LOG.info("Kafka Replication Factor: " + numOfReplicas);
        return props;
    }

    public Properties propertiesForZooKeeper() {
        Properties props = getPropertiesFile(this.getPropertiesFileZooKeeper(), "ZooKeeper");

        AteDelegate d = AteDelegate.get();
        if (d.bootstrapConfig.getZookeeperDataDirOverride() != null) {
            props.put("dataDir", d.bootstrapConfig.getZookeeperDataDirOverride());
        }

        return props;
    }

    public Properties propertiesForToken() {
        return getPropertiesFile(this.getPropertiesFileToken(), "Token");
    }

    public static String propertyOrThrow(Properties props, String name) {
        AteDelegate d = AteDelegate.get();
        if (props == d.bootstrapConfig.propertiesForAte()) {
            if ("zookeeper.bootstrap".equals(name)) {
                String override = d.bootstrapConfig.getBootstrapOverrideZookeeper();
                if (override != null) return override;
            }
            if ("kafka.bootstrap".equals(name)) {
                String override = d.bootstrapConfig.getBootstrapOverrideKafka();
                if (override != null) return override;
            }
        }

        String ret = props.getProperty(name, null);
        if (ret == null) {
            throw new RuntimeException("Unable to initialize the subsystem as the [" + name + "] is missing from the properties file.");
        }
        return ret;
    }

    public String getImplicitAuthorityAlias() {
        return implicitAuthorityAlias;
    }

    public void setImplicitAuthorityAlias(String implicitAuthorityAlias) {
        this.implicitAuthorityAlias = implicitAuthorityAlias;
    }

    public List<KeyType> getDefaultSigningTypes() {
        return securityLevel.signingTypes;
    }

    public List<KeyType> getDefaultEncryptTypes() {
        return securityLevel.encryptTypes;
    }

    public int getDefaultAesStrength() {
        return securityLevel.aesStrength;
    }

    public int getDefaultSigningStrength() {
        return securityLevel.signingStrength;
    }

    public int getDefaultEncryptionStrength() {
        return securityLevel.encryptionStrength;
    }

    public boolean getDefaultAutomaticKeyRotation() { return securityLevel.automaticKeyRotation; }

    public SecurityLevel getSecurityLevel() {
        return securityLevel;
    }

    public void setSecurityLevel(SecurityLevel securityLevel) {
        this.securityLevel = securityLevel;
    }

    public boolean isLoggingChainOfTrust() {
        return loggingChainOfTrust;
    }

    public void setLoggingChainOfTrust(boolean loggingChainOfTrust) {
        this.loggingChainOfTrust = loggingChainOfTrust;
    }

    public boolean isLoggingMessages() {
        return loggingMessages;
    }

    public void setLoggingMessages(boolean loggingMessages) {
        this.loggingMessages = loggingMessages;
    }

    public boolean isLoggingWrites() {
        return loggingWrites;
    }

    public void setLoggingWrites(boolean loggingWrites) {
        this.loggingWrites = loggingWrites;
    }

    public boolean isLoggingReads() {
        return loggingReads;
    }

    public void setLoggingReads(boolean loggingReads) {
        this.loggingReads = loggingReads;
    }

    public boolean isLoggingData() {
        return loggingData;
    }

    public void setLoggingData(boolean loggingData) {
        this.loggingData = loggingData;
    }

    public boolean isLoggingDeletes() {
        return loggingDeletes;
    }

    public void setLoggingDeletes(boolean loggingDeletes) {
        this.loggingDeletes = loggingDeletes;
    }

    public boolean isLoggingKafka() {
        return loggingKafka;
    }

    public void setLoggingKafka(boolean loggingKafka) {
        this.loggingKafka = loggingKafka;
    }

    public boolean isLoggingWithStackTrace() {
        return loggingWithStackTrace;
    }

    public void setLoggingWithStackTrace(boolean loggingWithStackTrace) {
        this.loggingWithStackTrace = loggingWithStackTrace;
    }

    public DefaultStorageSystem getDefaultStorageSystem() {
        return defaultStorageSystem;
    }

    public void setDefaultStorageSystem(DefaultStorageSystem defaultStorageSystem) {
        this.defaultStorageSystem = defaultStorageSystem;
    }

    public boolean isLoggingSync() {
        return loggingSync;
    }

    public void setLoggingSync(boolean loggingSync) {
        this.loggingSync = loggingSync;
    }

    public boolean isLoggingValidationVerbose() {
        return loggingValidationVerbose;
    }

    public void setLoggingValidationVerbose(boolean loggingValidationVerbose) {
        this.loggingValidationVerbose = loggingValidationVerbose;
    }

    public boolean isExtraValidation() {
        return extraValidation;
    }

    public void setExtraValidation(boolean extraValidation) {
        this.extraValidation = extraValidation;
    }

    public String getDnsServer() {
        return dnsServer;
    }

    public void setDnsServer(String dnsServer) {
        this.dnsServer = dnsServer;
        AteDelegate.get().implicitSecurity.init();
    }

    public String getBootstrapOverrideZookeeper() {
        return bootstrapOverrideZookeeper;
    }

    public void setBootstrapOverrideZookeeper(String bootstrapOverrideZookeeper) {
        this.bootstrapOverrideZookeeper = bootstrapOverrideZookeeper;
    }

    public String getBootstrapOverrideKafka() {
        return bootstrapOverrideKafka;
    }

    public void setBootstrapOverrideKafka(String bootstrapOverrideKafka) {
        this.bootstrapOverrideKafka = bootstrapOverrideKafka;
    }

    public String getKafkaLogDirOverride() {
        return kafkaLogDirOverride;
    }

    public void setKafkaLogDirOverride(String kafkaLogDirOverride) {
        this.kafkaLogDirOverride = kafkaLogDirOverride;
    }

    public String getZookeeperDataDirOverride() {
        return zookeeperDataDirOverride;
    }

    public void setZookeeperDataDirOverride(String zookeeperDataDirOverride) {
        this.zookeeperDataDirOverride = zookeeperDataDirOverride;
    }

    public boolean isLoggingCallbacks() {
        return loggingCallbacks;
    }

    public void setLoggingCallbacks(boolean loggingCallbacks) {
        this.loggingCallbacks = loggingCallbacks;
    }

    public boolean isLoggingCallbackData() {
        return loggingCallbackData;
    }

    public void setLoggingCallbackData(boolean loggingCallbackData) {
        this.loggingCallbackData = loggingCallbackData;
    }

    public int getSubscriberPartitionTimeout() {
        return subscriberPartitionTimeout;
    }

    public void setSubscriberPartitionTimeout(int subscriberPartitionTimeout) {
        this.subscriberPartitionTimeout = subscriberPartitionTimeout;
    }

    public int getSubscriberMaxPartitions() {
        return subscriberMaxPartitions;
    }

    public void setSubscriberMaxPartitions(int subscriberMaxPartitions) {
        this.subscriberMaxPartitions = subscriberMaxPartitions;
    }

    public String getPropertiesFileToken() {
        return propertiesFileToken;
    }

    public void setPropertiesFileToken(String propertiesFileToken) {
        this.propertiesFileToken = propertiesFileToken;
    }

    public Integer getRestPortOverride() {
        return restPortOverride;
    }

    public void setRestPortOverride(Integer restPortOverride) {
        this.restPortOverride = restPortOverride;
    }

    public boolean isLoggingMessageData() {
        return loggingMessageData;
    }

    public void setLoggingMessageData(boolean loggingMessageData) {
        this.loggingMessageData = loggingMessageData;
    }

    public boolean isLoggingMessageDrops() {
        return loggingMessageDrops;
    }

    public void setLoggingMessageDrops(boolean loggingMessageDrops) {
        this.loggingMessageDrops = loggingMessageDrops;
    }

    public boolean isLoggingIoStackTraces() {
        return loggingIoStackTraces;
    }

    public void setLoggingIoStackTraces(boolean loggingIoStackTraces) {
        this.loggingIoStackTraces = loggingIoStackTraces;
    }

    public int getDataMaintenanceWindow() {
        return dataMaintenanceWindow;
    }

    public void setDataMaintenanceWindow(int dataMaintenanceWindow) {
        this.dataMaintenanceWindow = dataMaintenanceWindow;
    }

    public int getPermissionsCacheLimit() {
        return permissionsCacheLimit;
    }

    public void setPermissionsCacheLimit(int permissionsCacheLimit) {
        this.permissionsCacheLimit = permissionsCacheLimit;
    }

    public int getIndexingMaximumViewsPerTable() {
        return indexingMaximumViewsPerTable;
    }

    public void setIndexingMaximumViewsPerTable(int indexingMaximumViewsPerTable) {
        this.indexingMaximumViewsPerTable = indexingMaximumViewsPerTable;
    }

    public int getIndexingExpireDelay() {
        return indexingExpireDelay;
    }

    public void setIndexingExpireDelay(int indexingExpireDelay) {
        this.indexingExpireDelay = indexingExpireDelay;
    }

    public boolean isEnableAutomaticIndexing() {
        return enableAutomaticIndexing;
    }

    public void setEnableAutomaticIndexing(boolean enableAutomaticIndexing) {
        this.enableAutomaticIndexing = enableAutomaticIndexing;
    }
}
