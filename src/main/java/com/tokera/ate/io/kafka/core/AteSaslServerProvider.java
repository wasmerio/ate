package com.tokera.ate.io.kafka.core;

import java.security.Provider;
import java.security.Security;

public class AteSaslServerProvider extends Provider {

    private static final long serialVersionUID = 1L;

    @SuppressWarnings("deprecation")
    protected AteSaslServerProvider() {
        super("Simple SASL/PLAIN Server Provider", 1.0, "Simple SASL/PLAIN Server Provider for Kafka");
        put("SaslServerFactory." + AteSaslServer.ATE_MECHANISM, AteSaslServer.AteSaslServerFactory.class.getName());
    }

    public static void initialize() {
        Security.addProvider(new AteSaslServerProvider());
    }
}
