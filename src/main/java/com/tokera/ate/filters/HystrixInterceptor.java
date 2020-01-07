package com.tokera.ate.filters;

import com.netflix.hystrix.strategy.concurrency.HystrixRequestContext;
import com.netflix.hystrix.strategy.concurrency.HystrixRequestVariableDefault;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.delegates.CurrentTokenDelegate;
import com.tokera.ate.delegates.RequestContextDelegate;
import com.tokera.ate.scopes.IScope;
import com.tokera.ate.scopes.IScopeContext;
import io.smallrye.faulttolerance.CommandListener;
import io.smallrye.faulttolerance.config.FaultToleranceOperation;
import org.jboss.resteasy.spi.ResteasyProviderFactory;
import org.jboss.weld.context.bound.BoundRequestContext;
import org.jboss.weld.context.http.HttpRequestContext;
import org.jboss.weld.contexts.AbstractBoundContext;
import org.jboss.weld.contexts.beanstore.BoundBeanStore;
import org.jboss.weld.module.web.context.http.HttpRequestContextImpl;

import javax.annotation.Priority;
import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.context.RequestScoped;
import javax.enterprise.context.spi.AlterableContext;
import javax.enterprise.inject.Instance;
import javax.enterprise.inject.spi.Bean;
import javax.enterprise.inject.spi.BeanManager;
import javax.enterprise.inject.spi.CDI;
import javax.inject.Inject;
import javax.servlet.http.HttpServletRequest;
import javax.servlet.http.HttpServletResponse;
import javax.ws.rs.container.ContainerRequestContext;
import javax.ws.rs.container.ContainerRequestFilter;
import javax.ws.rs.container.ContainerResponseContext;
import javax.ws.rs.container.ContainerResponseFilter;
import javax.ws.rs.core.Context;
import javax.ws.rs.ext.Provider;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.*;
import java.util.stream.Collectors;

/**
 * This interceptor initializes the hystrix request scope
 */
@ApplicationScoped
@Provider
@Priority(Integer.MAX_VALUE)
public class HystrixInterceptor implements ContainerRequestFilter, ContainerResponseFilter, CommandListener {
    protected AteDelegate d = AteDelegate.get();

    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;
    @Inject
    @SuppressWarnings("initialization.fields.uninitialized")
    private HttpRequestContext httpRequestContext;

    private Method methodGetBeanStore;
    private Method methodSetBeanStore;
    private HystrixRequestVariableDefault<HystrixContext> hystrixContext = new HystrixRequestVariableDefault<>();

    public HystrixInterceptor() throws NoSuchMethodException {

        Class<?> clazz = HttpRequestContextImpl.class.getSuperclass();
        List<Method> methods = Arrays.stream(clazz.getDeclaredMethods()).collect(Collectors.toList());

        this.methodGetBeanStore = methods.stream().filter(m -> "getBeanStore".equals(m.getName())).findFirst().orElseThrow(() -> new RuntimeException("Missing getBeanStore method.,"));
        this.methodSetBeanStore = methods.stream().filter(m -> "setBeanStore".equals(m.getName())).findFirst().orElseThrow(() -> new RuntimeException("Missing getBeanStore method.,"));

        this.methodGetBeanStore.setAccessible(true);
        this.methodSetBeanStore.setAccessible(true);
    }

    public static class HystrixContext {
        public HystrixRequestContext hystrixRequestContext;
        public HttpServletRequest httpServletRequest;
        public BoundBeanStore beanStore;
        public Map<Class<?>, Object> contextDataMap;
        public Map<IScopeContext, IScope> otherScopes = new HashMap<>();
        public boolean scopedUpdated = false;
    }

    @SuppressWarnings({"unchecked"})
    @Override
    public void filter(ContainerRequestContext containerRequestContext) {
        HystrixRequestContext hystrixRequestContext;
        if (HystrixRequestContext.isCurrentThreadInitialized() == false) {
            hystrixRequestContext = HystrixRequestContext.initializeContext();
        } else {
            hystrixRequestContext = HystrixRequestContext.getContextForCurrentThread();
        }

        HystrixContext myContext = new HystrixContext();
        this.hystrixContext.set(myContext);

        myContext.hystrixRequestContext = hystrixRequestContext;
        myContext.httpServletRequest = ResteasyProviderFactory.getContextData(HttpServletRequest.class);
        myContext.contextDataMap = ResteasyProviderFactory.getContextDataMap();
        try {
            myContext.beanStore = (BoundBeanStore)methodGetBeanStore.invoke(httpRequestContext);
        } catch (IllegalAccessException | InvocationTargetException e) {
            LOG.warn(e);
            myContext.beanStore = null;
        }

        d.requestContext.setHystrixContext(myContext);

        for (IScopeContext scopeContext : d.scopeContext.getScopeContexts()) {
            IScope scope = scopeContext.getLocalWithInactive();
            myContext.otherScopes.put(scopeContext, scope);
        }
    }

    @Override
    public void filter(ContainerRequestContext containerRequestContext, ContainerResponseContext containerResponseContext)
    {
        HystrixContext myContext = null;
        if (RequestContextDelegate.isWithinRequestContext()) {
            myContext = d.requestContext.getHystrixContext();
        }
        if (myContext != null) {
            if (myContext.scopedUpdated) {
                myContext.otherScopes.forEach((c, s) -> c.setLocal(s));
            }
        }

        if (HystrixRequestContext.isCurrentThreadInitialized()) {
            HystrixRequestContext hystrixRequestContext = HystrixRequestContext.getContextForCurrentThread();
            hystrixRequestContext.shutdown();
        }
    }

    @Override
    public void beforeExecution(FaultToleranceOperation operation) {
        HystrixContext myContext = hystrixContext.get();
        if (myContext == null) return;

        this.httpRequestContext.associate(myContext.httpServletRequest);
        this.httpRequestContext.activate();

        if (myContext.beanStore != null) {
            try {
                methodSetBeanStore.invoke(httpRequestContext, myContext.beanStore);
            } catch (Throwable e) {
                LOG.warn(e);
            }
        }

        myContext.otherScopes.forEach((c, s) -> c.setLocal(s));

        ResteasyProviderFactory.pushContextDataMap(myContext.contextDataMap);
    }

    @Override
    public void afterExecution(FaultToleranceOperation operation) {
        HystrixContext myContext = hystrixContext.get();

        if (myContext != null) {
            for (IScopeContext scopeContext : d.scopeContext.getScopeContexts()) {
                IScope scope = scopeContext.getLocalWithInactive();
                myContext.otherScopes.put(scopeContext, scope);
            }
            myContext.scopedUpdated = true;
        }

        /*
        try {
            this.httpRequestContext.invalidate();
            this.httpRequestContext.deactivate();
        }
        finally {
            if (myContext != null) {
                this.httpRequestContext.dissociate(myContext.httpServletRequest);
            }
        }
        */
    }
}
