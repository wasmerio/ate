package com.tokera.examples;

import javax.ws.rs.ApplicationPath;
import com.tokera.ate.ApiServer;
import com.tokera.ate.BootstrapApp;
import com.tokera.ate.BootstrapConfig;

@ApplicationPath("1-0")
public class HelloWorldApp extends BootstrapApp {

    public HelloWorldApp() { }

    public static void main(String[] args) {
        start(args);
    }

    public static void start(String[] args) {
        BootstrapConfig config = ApiServer.startWeld(args);
        config.setDeploymentName("HelloWorld API");

        ApiServer.startApiServer(config);
    }
}