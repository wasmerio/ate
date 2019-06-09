package com.tokera.ate;

import com.google.common.collect.Lists;
import com.tokera.ate.common.ApplicationConfigLoader;
import com.tokera.ate.configuration.AteConstants;
import com.tokera.ate.dao.enumerations.KeyType;
import com.tokera.ate.scopes.Startup;
import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.WebApplicationException;
import java.util.ArrayList;
import java.util.List;
import java.util.Properties;

@Startup
@ApplicationScoped
public class BootstrapConfig {
    private Class<?> applicationClass = BootstrapApp.class;
    private String restApiPath = "/rs";
    private String deploymentName = "MyAPI";
    private String pingCheckUrl = "login/ping";
    private String implicitAuthorityAlias = "auth";
    private boolean pingCheckOnStart = false;
    private String stsVaultFilename = "/token.signing.jks";
    private String stsVaultPassword = "7E264A281750DBEA5F15269D47AF1003877426D5EF7F99C4E739E0C9942C58470F15E678C32FB99B";
    private String stsSigningKeyPassword = "F4257978B79904B78903AB62C3B9F7EBFF42FDC8ED1F66995584DCD4D9E27E1082563FE92D7078A4";
    private String stsCertificateAliasName = "sts";

    private List<String> arguments = new ArrayList<>();

    private String propertiesFileAte = "ate.properties";
    private String propertiesFileLog4j = "log4j.properties";
    private String propertiesFileKafka = "kafka.properties";
    private String propertiesFileZooKeeper = "zookeeper.properties";
    private String propertiesFileConsumer = "consumer.properties";
    private String propertiesFileProducer = "producer.properties";
    private String propertiesFileTopicDao = "topic.dao.properties";
    private String propertiesFileTopicIo = "topic.io.properties";
    private String propertiesFileTopicPublic = "topic.publish.properties";

    private SecurityLevel securityLevel = SecurityLevel.HighlySecure;

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

    public String getStsVaultFilename() {
        return stsVaultFilename;
    }

    public void setStsVaultFilename(String stsVaultFilename) {
        this.stsVaultFilename = stsVaultFilename;
    }

    public String getStsVaultPassword() {
        return stsVaultPassword;
    }

    public void setStsVaultPassword(String stsVaultPassword) {
        this.stsVaultPassword = stsVaultPassword;
    }

    public String getStsSigningKeyPassword() {
        return stsSigningKeyPassword;
    }

    public void setStsSigningKeyPassword(String stsSigningKeyPassword) {
        this.stsSigningKeyPassword = stsSigningKeyPassword;
    }

    public String getStsCertificateAliasName() {
        return stsCertificateAliasName;
    }

    public void setStsCertificateAliasName(String stsCertificateAliasName) {
        this.stsCertificateAliasName = stsCertificateAliasName;
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
        Properties props = ApplicationConfigLoader.getInstance().getPropertiesByName(filename);
        if (props == null) {
            throw new RuntimeException("Properties file (" + filename + ") for " + logicalName + " does not exist.");
        }
        return props;
    }

    public Properties propertiesForAte() {
        return getPropertiesFile(this.getPropertiesFileAte(), "ATE");
    }

    public Properties propertiesForKafka() {
        return getPropertiesFile(this.getPropertiesFileKafka(), "Kafka");
    }

    public Properties propertiesForZooKeeper() {
        return getPropertiesFile(this.getPropertiesFileZooKeeper(), "ZooKeeper");
    }

    public static String propertyOrThrow(Properties props, String name) {
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

    public Iterable<KeyType> getDefaultSigningTypes() {
        return securityLevel.signingTypes;
    }

    public Iterable<KeyType> getDefaultEncryptTypes() {
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
}
