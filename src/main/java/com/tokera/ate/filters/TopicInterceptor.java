package com.tokera.ate.filters;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.delegates.AteDelegate;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import javax.ws.rs.container.ContainerRequestContext;
import javax.ws.rs.container.ContainerRequestFilter;
import javax.ws.rs.ext.Provider;
import javax.annotation.Priority;
import javax.ws.rs.Priorities;

/**
 * Filter that reads the Topic header from the request and uses this to build a topic scope.
 */
@ApplicationScoped
@Provider
@Priority(Priorities.AUTHENTICATION)
public class TopicInterceptor implements ContainerRequestFilter {

    protected AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private DefaultBootstrapInit interceptorInit;
    
    public static final String HEADER_TOPIC = "Topic";

    @PostConstruct
    public void init() {
        interceptorInit.touch();
    }

    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible"})       // We want to return a null if the data does not exist and it must be atomic
    private static @Nullable String getHeaderStringOrNull(ContainerRequestContext requestContext, String s) {
        return requestContext.getHeaderString(s);
    }

    @Override
    public void filter(ContainerRequestContext requestContext)
    {
        // Set the requestContext variable
        d.requestContext.setContainerRequestContext(requestContext);

        // Extract the token (either from the authorization header)
        String topicString = TopicInterceptor.getHeaderStringOrNull(requestContext, HEADER_TOPIC);
        if (topicString != null) {
            d.requestContext.pushTopicScope(topicString);
            
            // We warm up the topic locally as it is likely that there will be
            // IO very soon to this repository
            d.headIO.warm();
        }
    }
}
