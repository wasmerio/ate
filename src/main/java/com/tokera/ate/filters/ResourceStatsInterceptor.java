package com.tokera.ate.filters;

import com.tokera.ate.delegates.AteDelegate;

import javax.annotation.Priority;
import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.container.ContainerRequestContext;
import javax.ws.rs.container.ContainerRequestFilter;
import javax.ws.rs.ext.Provider;

/**
 * This interceptor provides updates the method statistics
 * @author John Sharratt (johnathan.sharratt@gmail.com)
 */
@ApplicationScoped
@Provider
@Priority(5100)
public class ResourceStatsInterceptor implements ContainerRequestFilter {

    protected AteDelegate d = AteDelegate.get();

    @Override
    public void filter(ContainerRequestContext requestContext) {
        if (d.resourceScopeInterceptor.isActive()) {
            d.resourceStats.add();
        }
    }
}
