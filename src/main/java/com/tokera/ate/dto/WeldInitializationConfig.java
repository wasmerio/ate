package com.tokera.ate.dto;

import com.esotericsoftware.yamlbeans.YamlReader;
import com.tokera.ate.BootstrapApp;
import com.tokera.ate.KafkaServer;
import com.tokera.ate.ZooServer;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.extensions.*;
import com.tokera.ate.providers.ProcessBodyReader;
import com.tokera.ate.providers.ProcessBodyWriter;
import com.tokera.ate.providers.TokeraResteasyJackson2Provider;
import com.tokera.ate.providers.YamlProvider;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.jboss.resteasy.cdi.ResteasyCdiExtension;
import org.jboss.resteasy.plugins.providers.StringTextStar;
import org.jboss.resteasy.plugins.providers.html.HtmlRenderableWriter;
import org.jboss.weld.bootstrap.spi.BeanDiscoveryMode;

import javax.enterprise.inject.spi.Extension;
import java.util.LinkedHashSet;
import java.util.LinkedList;

public class WeldInitializationConfig<T extends BootstrapApp> {

    public boolean enableDiscovery = true;
    public BeanDiscoveryMode discoveryMode = BeanDiscoveryMode.ANNOTATED;
    public final String @Nullable [] args;
    public final Class<T> clazz;
    public final LinkedHashSet<Class<?>> packages = new LinkedHashSet<>();
    public final LinkedList<Extension> extensions = new LinkedList<>();
    public final LinkedHashSet<Class<?>> beanClasses = new LinkedHashSet<>();

    public WeldInitializationConfig(String @Nullable [] _args, Class<T> clazz)
    {
        this.args = _args;
        this.clazz = clazz;

        /*
        this.packages.add(YamlReader.class);
        this.packages.add(HtmlRenderableWriter.class);
        this.packages.add(StringTextStar.class);
        this.packages.add(TokeraResteasyJackson2Provider.class);
        this.packages.add(AteDelegate.class);

        this.extensions.add(new ResteasyCdiExtension());
        this.extensions.add(new YamlTagDiscoveryExtension());
        this.extensions.add(new DaoParentDiscoveryExtension());
        this.extensions.add(new StartupBeanExtension());
        this.extensions.add(new ResourceScopedExtension());
        this.extensions.add(new TokenScopeExtension());
        this.extensions.add(new SerializableObjectsExtension());
        this.extensions.add(new io.smallrye.faulttolerance.HystrixExtension());

        this.beanClasses.add(YamlProvider.class);
        this.beanClasses.add(ProcessBodyReader.class);
        this.beanClasses.add(ProcessBodyWriter.class);
        this.beanClasses.add(TokeraResteasyJackson2Provider.class);
        this.beanClasses.add(ZooServer.class);
        this.beanClasses.add(KafkaServer.class);
        this.beanClasses.add(LoggerHook.class);
        */
    }

    public WeldInitializationConfig<T> clearPackages() {
        this.packages.clear();
        return this;
    }

    public WeldInitializationConfig<T> clearExtensions() {
        this.extensions.clear();
        return this;
    }

    public WeldInitializationConfig<T> clearBeanClasses() {
        this.beanClasses.clear();
        return this;
    }

    public WeldInitializationConfig<T> addBeanClass(Class<?> clazz) {
        this.beanClasses.add(clazz);
        return this;
    }

    public WeldInitializationConfig<T> enableDiscovery() {
        this.enableDiscovery = true;
        return this;
    }

    public WeldInitializationConfig<T> disableDiscovery() {
        this.enableDiscovery = false;
        return this;
    }
}
