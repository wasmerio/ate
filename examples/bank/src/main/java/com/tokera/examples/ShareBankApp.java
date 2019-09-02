package com.tokera.examples;

import com.tokera.ate.ApiServer;
import com.tokera.ate.BootstrapApp;
import com.tokera.ate.BootstrapConfig;
import com.tokera.ate.enumerations.DefaultStorageSystem;

import javax.ws.rs.ApplicationPath;

@ApplicationPath("1-0")
public class ShareBankApp extends BootstrapApp {

    public ShareBankApp() { }

    public static void main(String[] args) {
        run(args, DefaultStorageSystem.Kafka);
    }

    public static void run(String[] args, DefaultStorageSystem storage) {
        BootstrapConfig config = ApiServer.startWeld(args, ShareBankApp.class);
        config.setDeploymentName("ShareBank");
        config.setLoggingChainOfTrust(true);
        config.setLoggingWrites(true);
        config.setDefaultStorageSystem(storage);
        config.setExtraValidation(true);

        ApiServer.startApiServer(config);
    }
}