package com.tokera.ate;

import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.inject.spi.Extension;

@ApplicationScoped
public class BootstrapConfig implements Extension {
    public Class<?> applicationClass = BootstrapApp.class;
    public String propertiesFile = "ate.properties";
    public String restApiPath = "/rs";
    public String deploymentName = "MyAPI";
    public String domain = "mydomain.com";
    public String zookeeperAlias = "tokkeep";
    public String kafkaAlias = "tokdata";
    public String implicitSecurityAlias = "tokauth";
    public String pingCheckUrl = "login/ping";
    public boolean pingCheckOnStart = false;
    public String stsVaultFilename = "/token.signing.jks";
    public String stsVaultPassword = "7E264A281750DBEA5F15269D47AF1003877426D5EF7F99C4E739E0C9942C58470F15E678C32FB99B";
    public String stsSigningKeyPassword = "F4257978B79904B78903AB62C3B9F7EBFF42FDC8ED1F66995584DCD4D9E27E1082563FE92D7078A4";
    public String stsCertificateAliasName = "sts";
}
