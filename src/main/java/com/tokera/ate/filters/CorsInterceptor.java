package com.tokera.ate.filters;

import com.tokera.ate.scopes.Startup;
import org.checkerframework.checker.nullness.qual.Nullable;

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
@Startup
@ApplicationScoped
@Provider
@Priority(Priorities.HEADER_DECORATOR)
public class CorsInterceptor implements ContainerResponseFilter {

    public static final String ORIGIN_STRING = "Origin";
    public static final String ACCESS_CONTROL_REQUEST_METHOD_STRING = "Access-Control-Request-Method";
    public static final String ACCESS_CONTROL_REQUEST_HEADERS_STRING = "Access-Control-Request-Headers";

    public static final String ACCESS_CONTROL_ALLOW_ORIGIN_STRING = "Access-Control-Allow-Origin";
    public static final String ACCESS_CONTROL_ALLOW_CREDENTIALS_STRING = "Access-Control-Allow-Credentials";
    public static final String ACCESS_CONTROL_EXPOSE_HEADERS_STRING = "Access-Control-Expose-Headers";
    public static final String ACCESS_CONTROL_MAX_AGE_STRING = "Access-Control-Max-Age";
    public static final String ACCESS_CONTROL_ALLOW_METHODS_STRING = "Access-Control-Allow-Methods";
    public static final String ACCESS_CONTROL_ALLOW_HEADERS_STRING = "Access-Control-Allow-Headers";

    public static final String ORIGIN = "Origin";
    public static final String ACCESS_CONTROL_REQUEST_METHOD = "Access-Control-Request-Method";
    public static final String ACCESS_CONTROL_REQUEST_HEADERS = "Access-Control-Request-Headers";

    public static final String ACCESS_CONTROL_ALLOW_ORIGIN = "Access-Control-Allow-Origin";
    public static final String ACCESS_CONTROL_ALLOW_CREDENTIALS = "Access-Control-Allow-Credentials";
    public static final String ACCESS_CONTROL_EXPOSE_HEADERS = "Access-Control-Expose-Headers";
    public static final String ACCESS_CONTROL_MAX_AGE = "Access-Control-Max-Age";
    public static final String ACCESS_CONTROL_ALLOW_METHODS = "Access-Control-Allow-Methods";
    public static final String ACCESS_CONTROL_ALLOW_HEADERS = "Access-Control-Allow-Headers";

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

        String origin = getHeaderStringOrNull(requestContext, CorsInterceptor.ORIGIN);
        if (origin == null || requestContext.getMethod().equalsIgnoreCase("OPTIONS") || getPropertyOrNull(requestContext, "cors.failure") != null)
        {
            // don't do anything if origin is null, its an OPTIONS currentRights, or cors.failure is set
            return;
        }

        final MultivaluedMap<String, Object> headers = responseContext.getHeaders();
        headers.add(CorsInterceptor.ACCESS_CONTROL_ALLOW_ORIGIN, "*");
        headers.add(CorsInterceptor.ACCESS_CONTROL_ALLOW_HEADERS, "Authorization, Origin, X-Requested-With, Content-Type, Topic, NodeId, Dba");
        headers.add(CorsInterceptor.ACCESS_CONTROL_ALLOW_METHODS, "OPTIONS, GET, POST, DELETE, PUT, PATCH, HEAD");
        headers.add(CorsInterceptor.ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
        headers.add(CorsInterceptor.ACCESS_CONTROL_EXPOSE_HEADERS, "Location, Content-Disposition, Authorization, ApiVersion, Track, Invalidate");
    }
}