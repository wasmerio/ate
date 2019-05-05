package com.tokera.examples;

import javax.ws.rs.ApplicationPath;
import com.tokera.ate.ApiServer;
import com.tokera.ate.BootstrapApp;
import com.tokera.ate.BootstrapConfig;
import com.tokera.server.api.configuration.TokeraConstant;
import com.tokera.server.api.configuration.TokeraProperty;
import com.tokera.server.api.terminal.TerminalWebSocket;
import io.undertow.servlet.api.DeploymentInfo;
import io.undertow.websockets.jsr.WebSocketDeploymentInfo;
import static io.undertow.servlet.Servlets.deployment;

@ApplicationPath("1-0")
public class HelloWorldApp extends BootstrapApp {

    public HelloWorldApp() { }

    public static void main(String[] args) {
        start();
    }

    public static void start() {
        BootstrapConfig config = ApiServer.startWeld();
        config.setApplicationClass(MainApp.class);
        config.setDeploymentName("Example API");
        config.setRestApiPath("/rs");
        config.setPropertiesFile("example.configuration");
        config.setDomain("examples.tokera.com");
        config.setPingCheckOnStart(true);
        ApiServer apiServer = ApiServer.startApiServer(config);
    }
}