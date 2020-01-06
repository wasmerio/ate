package com.tokera.ate.test.rest.api;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.scopes.Startup;

import javax.annotation.Priority;
import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.container.ContainerRequestContext;
import javax.ws.rs.container.ContainerRequestFilter;
import javax.ws.rs.ext.Provider;

@ApplicationScoped
@Startup
@Provider
@Priority(5000)
public class MySpecialRequestDelegate implements ContainerRequestFilter {
    protected AteDelegate d = AteDelegate.get();

    @SuppressWarnings({"unchecked"})
    @Override
    public void filter(ContainerRequestContext containerRequestContext) {
        d.requestContext.setCustomData("my-data");
    }
}
