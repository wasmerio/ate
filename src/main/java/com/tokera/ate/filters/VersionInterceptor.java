package com.tokera.ate.filters;

import com.tokera.ate.common.ApplicationConfigLoader;
import com.tokera.ate.scopes.Startup;

import javax.annotation.Priority;
import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.Priorities;
import javax.ws.rs.container.ContainerRequestContext;
import javax.ws.rs.container.ContainerResponseContext;
import javax.ws.rs.container.ContainerResponseFilter;
import javax.ws.rs.core.MultivaluedMap;
import javax.ws.rs.ext.Provider;

@Startup
@ApplicationScoped
@Provider
@Priority(Priorities.HEADER_DECORATOR)
public class VersionInterceptor implements ContainerResponseFilter {

    public static final String HEADER_VERSION = "ApiVersion";

    @Override
    public void filter(ContainerRequestContext requestContext, ContainerResponseContext responseContext) {        
        final MultivaluedMap<String, Object> headers = responseContext.getHeaders();
        headers.add(HEADER_VERSION, ApplicationConfigLoader.getCurrentVersion());
    }
}
