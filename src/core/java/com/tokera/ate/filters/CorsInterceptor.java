package com.tokera.ate.filters;

import org.checkerframework.checker.nullness.qual.Nullable;
import org.jboss.resteasy.spi.CorsHeaders;

import javax.annotation.Priority;
import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.Priorities;
import javax.ws.rs.container.ContainerRequestContext;
import javax.ws.rs.container.ContainerResponseContext;
import javax.ws.rs.container.ContainerResponseFilter;
import javax.ws.rs.core.MultivaluedMap;
import javax.ws.rs.ext.Provider;

/**
 * Adds Cors headers so that APIs play nice with browsers
 * @author jonhanlee
 */
@ApplicationScoped
@Provider
@Priority(Priorities.HEADER_DECORATOR)
public class CorsInterceptor implements ContainerResponseFilter {

    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible"})       // We want to return a null if the data does not exist and it must be atomic
    private static @Nullable String getHeaderStringOrNull(ContainerRequestContext requestContext, String s) {
        return requestContext.getHeaderString(s);
    }

    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible"})       // We want to return a null if the data does not exist and it must be atomic
    private static @Nullable Object getPropertyOrNull(ContainerRequestContext requestContext, String s) {
        return requestContext.getProperty(s);
    }
    
    @Override
    public void filter(ContainerRequestContext requestContext, ContainerResponseContext responseContext) {

        String origin = getHeaderStringOrNull(requestContext, CorsHeaders.ORIGIN);
        if (origin == null || requestContext.getMethod().equalsIgnoreCase("OPTIONS") || getPropertyOrNull(requestContext, "cors.failure") != null)
        {
            // don't do anything if origin is null, its an OPTIONS currentRights, or cors.failure is set
            return;
        }

        final MultivaluedMap<String, Object> headers = responseContext.getHeaders();
        headers.add(CorsHeaders.ACCESS_CONTROL_ALLOW_ORIGIN, "*");
        headers.add(CorsHeaders.ACCESS_CONTROL_ALLOW_HEADERS, "Authorization, Origin, X-Requested-With, Content-Type, Topic, NodeId, Dba");
        headers.add(CorsHeaders.ACCESS_CONTROL_ALLOW_METHODS, "OPTIONS, GET, POST, DELETE, PUT, PATCH, HEAD");
        headers.add(CorsHeaders.ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
        headers.add(CorsHeaders.ACCESS_CONTROL_EXPOSE_HEADERS, "Location, Content-Disposition, Authorization, ApiVersion, Track, Invalidate");
    }
}