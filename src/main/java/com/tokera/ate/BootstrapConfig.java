package com.tokera.ate;

import com.tokera.ate.scopes.Startup;
import javax.enterprise.context.ApplicationScoped;

@Startup
@ApplicationScoped
public class BootstrapConfig {
    private Class<?> applicationClass = BootstrapApp.class;
    private String propertiesFile = "ate.properties";
    private String restApiPath = "/rs";
    private String deploymentName = "MyAPI";
    private String domain = "mydomain.com";
    private String zookeeperAlias = "tokkeep";
    private String kafkaAlias = "tokdata";
    private String implicitSecurityAlias = "tokauth";
    private String pingCheckUrl = "login/ping";
    private boolean pingCheckOnStart = false;
    private String stsVaultFilename = "/token.signing.jks";
    private String stsVaultPassword = "7E264A281750DBEA5F15269D47AF1003877426D5EF7F99C4E739E0C9942C58470F15E678C32FB99B";
    private String stsSigningKeyPassword = "F4257978B79904B78903AB62C3B9F7EBFF42FDC8ED1F66995584DCD4D9E27E1082563FE92D7078A4";
    private String stsCertificateAliasName = "sts";

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

    public String getPropertiesFile() {
        return propertiesFile;
    }

    public void setPropertiesFile(String propertiesFile) {
        this.propertiesFile = propertiesFile;
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

    public String getDomain() {
        return domain;
    }

    public void setDomain(String domain) {
        this.domain = domain;
    }

    public String getZookeeperAlias() {
        return zookeeperAlias;
    }

    public void setZookeeperAlias(String zookeeperAlias) {
        this.zookeeperAlias = zookeeperAlias;
    }

    public String getKafkaAlias() {
        return kafkaAlias;
    }

    public void setKafkaAlias(String kafkaAlias) {
        this.kafkaAlias = kafkaAlias;
    }

    public String getImplicitSecurityAlias() {
        return implicitSecurityAlias;
    }

    public void setImplicitSecurityAlias(String implicitSecurityAlias) {
        this.implicitSecurityAlias = implicitSecurityAlias;
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
}
