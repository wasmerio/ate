package com.tokera.ate.extensions;

import javax.ejb.Startup;
import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.event.Observes;
import javax.enterprise.inject.spi.*;
import javax.faces.bean.ManagedBean;
import java.util.LinkedHashSet;
import java.util.Set;

public class StartupBeanExtension implements Extension
{
    private final Set<Bean<?>> startupBeans = new LinkedHashSet<Bean<?>>();

    <X> void processBean(@Observes ProcessBean<X> event)
    {
        if (event.getAnnotated().isAnnotationPresent(ApplicationScoped.class))
        {
            if (event.getAnnotated().isAnnotationPresent(Startup.class)) {
                startupBeans.add(event.getBean());
            }
            if (event.getAnnotated().isAnnotationPresent(ManagedBean.class) &&
                event.getAnnotated().getAnnotation(ManagedBean.class).eager() == true) {
                startupBeans.add(event.getBean());
            }
        }
    }

    void afterDeploymentValidation(@Observes AfterDeploymentValidation event, BeanManager manager)
    {
        for (Bean<?> bean : startupBeans)
        {
            // the call to toString() is a cheat to force the bean to be initialized
            manager.getReference(bean, bean.getBeanClass(), manager.createCreationalContext(bean)).toString();
        }
    }
}