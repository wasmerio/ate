package com.tokera.ate.filters;

import com.tokera.ate.io.core.RequestAccessLog;
import com.tokera.ate.scopes.Startup;

import javax.annotation.Priority;
import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import javax.ws.rs.Priorities;
import javax.ws.rs.container.ContainerRequestContext;
import javax.ws.rs.container.ContainerResponseContext;
import javax.ws.rs.container.ContainerResponseFilter;
import javax.ws.rs.core.MultivaluedMap;
import javax.ws.rs.ext.Provider;

/**
 * Filter that will ensure Track and Invalidate headers are added to the response for any objects that are modified.
 * This tracking allows clients to build a local client-side cache that is eventually consistent within low latencies.
 */
@Startup
@ApplicationScoped
@Provider
@Priority(Priorities.HEADER_DECORATOR)
public class AccessLogInterceptor implements ContainerResponseFilter {

    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private RequestAccessLog accessLog;

    public static final String HEADER_TRACK_ID = "Track";
    public static final String HEADER_INVALIDATE_ID = "Invalidate";

    @Override
    public void filter(ContainerRequestContext requestContext, ContainerResponseContext responseContext) {
        
        // Add all the records we need to track in the TokFS engine
        MultivaluedMap<String, Object> headers = responseContext.getHeaders();
        for (String record : accessLog.getReadRecords()) {
            headers.add(HEADER_TRACK_ID, record);
        }
        for (String record : accessLog.getWroteRecords()) {
            headers.add(HEADER_INVALIDATE_ID, record);
        }
    }
}
