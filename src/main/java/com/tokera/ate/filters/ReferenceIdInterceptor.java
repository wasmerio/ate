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

@Startup
@ApplicationScoped
@Provider
@Priority(Priorities.HEADER_DECORATOR)
public class ReferenceIdInterceptor implements ContainerResponseFilter {

    public static final String HEADER_REFERENCE_ID = "ReferenceId";

    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible"})       // We want to return a null if the data does not exist and it must be atomic
    private static @Nullable String getHeaderStringOrNull(ContainerRequestContext requestContext, String s) {
        return requestContext.getHeaderString(s);
    }

    @Override
    public void filter(ContainerRequestContext requestContext, ContainerResponseContext responseContext) {
        // Pass the reference Id from the currentRights to the response
        String refId = ReferenceIdInterceptor.getHeaderStringOrNull(requestContext, HEADER_REFERENCE_ID);
        if (refId != null) {
            final MultivaluedMap<String, Object> headers = responseContext.getHeaders();
            headers.add(HEADER_REFERENCE_ID, refId);
        }
    }
}
