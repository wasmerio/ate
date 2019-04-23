package com.tokera.ate.filters;

import com.tokera.ate.common.LoggerHook;
import org.opensaml.DefaultBootstrap;
import org.opensaml.xml.ConfigurationException;

import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;

/**
 * Forces a bootstrap of the OpenSAML framework
 */
@ApplicationScoped
public class DefaultBootstrapInit {

    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;

    @PostConstruct
    public void init() {

        try {
            // Initialize the library
            DefaultBootstrap.bootstrap();
        } catch (ConfigurationException ex) {
            this.LOG.error("Bootstrap has failed", ex);
        }
    }

    public void touch() {
    }
}
