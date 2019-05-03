package com.tokera.ate.filters;

import org.apache.log4j.MDC;

import javax.annotation.Nullable;
import javax.annotation.Priority;
import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.container.ContainerRequestContext;
import javax.ws.rs.container.ContainerRequestFilter;
import javax.ws.rs.core.MultivaluedMap;
import javax.ws.rs.ext.Provider;
import java.util.Iterator;
import java.util.UUID;

@ApplicationScoped
@Provider
@Priority(5010)
public class LogReferenceInterceptor implements ContainerRequestFilter {

    private static @Nullable Object MDCget(String key) {
        return MDC.get(key);
    }
    
    @Override
    public void filter(ContainerRequestContext requestContext) {

        MultivaluedMap<String, String> pathParams = requestContext.getUriInfo().getPathParameters();
        Iterator<String> it = pathParams.keySet().iterator();
        while (it.hasNext()) {
            String theKey = it.next();
            String theValue = pathParams.getFirst(theKey);
            MDC.put(theKey, theValue);
        }
        
        if (LogReferenceInterceptor.MDCget("id") == null) {
            MDC.put("id", new UUID(0, 0).toString());
        }
    }
}
