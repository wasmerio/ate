package com.tokera.ate.filters;

import com.netflix.hystrix.strategy.concurrency.HystrixRequestContext;
import com.netflix.hystrix.strategy.concurrency.HystrixRequestVariableDefault;
import com.tokera.ate.delegates.AteDelegate;
import io.smallrye.faulttolerance.CommandListener;
import io.smallrye.faulttolerance.config.FaultToleranceOperation;
import org.jboss.weld.context.bound.BoundRequestContext;

import javax.annotation.Priority;
import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.inject.spi.CDI;
import javax.ws.rs.container.ContainerRequestContext;
import javax.ws.rs.container.ContainerRequestFilter;
import javax.ws.rs.container.ContainerResponseContext;
import javax.ws.rs.container.ContainerResponseFilter;
import javax.ws.rs.ext.Provider;
import java.util.Map;
import java.util.TreeMap;

/**
 * This interceptor initializes the hystrix request scope
 */
@ApplicationScoped
@Provider
@Priority(8000)
public class HystrixInterceptor implements ContainerRequestFilter, ContainerResponseFilter, CommandListener {

    protected AteDelegate d = AteDelegate.get();
    private HystrixRequestContext hystrixRequestContext = null;
    private HystrixRequestVariableDefault<Map<String, Object>> dataStoreVariable = new HystrixRequestVariableDefault<>();

    @SuppressWarnings({"unchecked"})
    @Override
    public void filter(ContainerRequestContext requestContext) {
        if (HystrixRequestContext.isCurrentThreadInitialized() == false) {
            this.hystrixRequestContext = HystrixRequestContext.initializeContext();
        }

        Map<String, Object> requestDataStore = new TreeMap<>();
        dataStoreVariable.set(requestDataStore);
    }

    @Override
    public void filter(ContainerRequestContext requestContext, ContainerResponseContext responseContext) {
        if (this.hystrixRequestContext != null) {
            this.hystrixRequestContext.shutdown();
        }
    }

    @Override
    public void beforeExecution(FaultToleranceOperation operation) {
        BoundRequestContext context = CDI.current().select(BoundRequestContext.class).get();
        if (context != null) {
            Map<String, Object> requestDataStore = dataStoreVariable.get();
            if (requestDataStore != null) {
                context.associate(requestDataStore);
            }
            context.activate();
        }
    }

    @Override
    public void afterExecution(FaultToleranceOperation operation) {
        BoundRequestContext context = CDI.current().select(BoundRequestContext.class).get();
        if (context != null) {
            context.invalidate();
            context.deactivate();
        }

        Map<String, Object> requestDataStore = dataStoreVariable.get();
        if (requestDataStore != null) {
            if (context != null) context.dissociate(requestDataStore);
            dataStoreVariable.remove();
        }
    }
}
