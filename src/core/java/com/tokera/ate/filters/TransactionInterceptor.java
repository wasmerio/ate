package com.tokera.ate.filters;

import com.tokera.ate.delegates.AteDelegate;

import java.io.IOException;
import javax.annotation.Priority;
import javax.enterprise.context.ApplicationScoped;

import javax.ws.rs.container.ContainerRequestContext;
import javax.ws.rs.container.ContainerResponseContext;
import javax.ws.rs.container.ContainerResponseFilter;
import javax.ws.rs.ext.Provider;

/**
 * Intercepter will merge all the data objects that were created during the call before it returns the response
 */
@ApplicationScoped
@Provider
@Priority(5100)
public class TransactionInterceptor implements ContainerResponseFilter {

    protected AteDelegate d = AteDelegate.get();

    @Override
    public void filter(ContainerRequestContext requestContext, ContainerResponseContext responseContext) throws IOException {
        
        // Send all the data to the Kafka
        // (but only if we are not encountering an error of some kind)
        if (responseContext.getStatus() >= 200 && responseContext.getStatus() < 400) {
            d.headIO.mergeDeferred();
        }
    }
}
