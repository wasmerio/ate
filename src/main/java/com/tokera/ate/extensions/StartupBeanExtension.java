package com.tokera.ate.extensions;

import com.tokera.ate.annotations.PermitParentType;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.scopes.ResourceScoped;
import com.tokera.ate.scopes.Startup;
import com.tokera.ate.scopes.TokenScoped;
import org.jboss.weld.environment.se.events.ContainerInitialized;

import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.context.Dependent;
import javax.enterprise.context.RequestScoped;
import javax.enterprise.context.spi.CreationalContext;
import javax.enterprise.event.Observes;
import javax.enterprise.inject.spi.*;
import java.lang.reflect.Field;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;

public class StartupBeanExtension implements Extension
{
    private final Map<Class<?>, Bean<?>> startupBeans = new ConcurrentHashMap<>();
    private final Map<Class<?>, Object> startupProxies = new ConcurrentHashMap<>();
    private final Set<Class<?>> already = new HashSet<>();

    void addBeans(@Observes BeforeBeanDiscovery discovery, BeanManager manager) {
        for (Field field : AteDelegate.class.getFields()) {
            Class<?> clazz = field.getType();
            if (clazz.isAnnotationPresent(ApplicationScoped.class) ||
                clazz.isAnnotationPresent(RequestScoped.class) ||
                clazz.isAnnotationPresent(TokenScoped.class) ||
                clazz.isAnnotationPresent(ResourceScoped.class) ||
                clazz.isAnnotationPresent(Dependent.class))
            {
                discovery.addAnnotatedType(manager.createAnnotatedType(clazz), clazz.getName());
            }
        }
    }

    <X> void processAnnotatedType(@Observes @WithAnnotations(Startup.class) ProcessAnnotatedType<X> pat) {
        Class<?> clazz = pat.getAnnotatedType().getJavaClass();
        if (clazz.getAnnotation(Startup.class) == null)  {
            return;
        }
        if (already.contains(clazz)) {
            pat.veto();
            return;
        }
        already.add(clazz);
    }

    <X> void processBean(@Observes ProcessBean<X> event)
    {
        if (event.getAnnotated().isAnnotationPresent(Startup.class))
        {
            if (event.getAnnotated().isAnnotationPresent(ApplicationScoped.class) == false) {
                throw new RuntimeException("All Startup beans must be marked with ApplicationScoped and the bean [" + event.getBean().getBeanClass() + "] is not.");
            }

            Bean<X> bean = event.getBean();
            startupBeans.put(bean.getBeanClass(), bean);
        }
    }

    void afterDeploymentValidation(@Observes AfterDeploymentValidation event, BeanManager manager)
    {
        for (Bean<?> bean : startupBeans.values()) {
            Class<?> clazz = bean.getBeanClass();
            CreationalContext<?> context = manager.createCreationalContext(bean);
            startupProxies.put(clazz, manager.getReference(bean, clazz, context));
        }

        AteDelegate.get().init();
    }

    public void start(@Observes final ContainerInitialized event) {
        // the call to toString() is a cheat to force the bean to be initialized
        for (Object obj : startupProxies.values()) {
            obj.toString();
        }

        AteDelegate.get().init();
    }
}