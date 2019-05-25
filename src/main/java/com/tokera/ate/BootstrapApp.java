package com.tokera.ate;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import javax.enterprise.event.Observes;
import javax.enterprise.inject.spi.Extension;
import javax.enterprise.inject.spi.ProcessAnnotatedType;
import javax.enterprise.inject.spi.WithAnnotations;
import javax.ws.rs.ApplicationPath;
import javax.ws.rs.Path;
import javax.ws.rs.core.Application;
import javax.ws.rs.ext.Provider;
import java.util.HashSet;
import java.util.Set;

@ApplicationPath("1-0")
public class BootstrapApp extends Application implements Extension {

    private static final Logger LOG = LoggerFactory.getLogger(BootstrapApp.class);

    private final Set<Class<?>> restEndpointClasses = new HashSet<>();
    private final Set<Class<?>> providerClasses = new HashSet<>();

    public BootstrapApp() { }
    
    @Override
    public Set<Class<?>> getClasses() {
        Set<Class<?>> resources = new HashSet<>();
        resources.addAll(providerClasses);
        resources.addAll(restEndpointClasses);
        return resources;
    }

    @Override
    public Set<Object> getSingletons() {
        Set<Object> ret = new HashSet<>();
        return ret;
    }
    
    public void watchForResources(@Observes @WithAnnotations(Path.class) ProcessAnnotatedType processAnnotatedType) {
        Class<?> resource = processAnnotatedType.getAnnotatedType().getJavaClass();
        LOG.info("BootstrapApp: Found Resource - " + resource.getName());
        restEndpointClasses.add(resource);
    }

    public void watchForProviders(@Observes @WithAnnotations(Provider.class) ProcessAnnotatedType processAnnotatedType) {
        Class<?> provider = processAnnotatedType.getAnnotatedType().getJavaClass();
        LOG.info("BootstrapApp: Found Provider - " + provider.getName());
        providerClasses.add(provider);
    }
}