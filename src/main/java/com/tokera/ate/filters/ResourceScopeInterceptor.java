package com.tokera.ate.filters;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.scopes.ResourceScoped;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;
import com.tokera.ate.scopes.ScopeContext;

import javax.annotation.Priority;
import javax.enterprise.context.RequestScoped;
import javax.ws.rs.container.*;
import javax.ws.rs.core.Context;
import javax.ws.rs.ext.Provider;
import java.lang.reflect.Method;

/**
 * This interceptor initializes the method scope for the currentRights user
 */
@RequestScoped
@Provider
@Priority(500)
public class ResourceScopeInterceptor implements ContainerRequestFilter, ContainerResponseFilter {

    protected AteDelegate d = AteDelegate.get();
    private @Context @Nullable ResourceInfo resourceInfo;
    private @MonotonicNonNull Method previous;
    private @MonotonicNonNull ScopeContext<Method> context;

    @SuppressWarnings({"unchecked"})
    @Override
    public void filter(ContainerRequestContext requestContext) {
        ResourceInfo resourceInfo = this.resourceInfo;
        if (resourceInfo == null) return;
        this.context = (ScopeContext<Method>) d.beanManager.getContext(ResourceScoped.class);
        this.previous = context.enter(resourceInfo.getResourceMethod());
    }

    @Override
    public void filter(ContainerRequestContext requestContext, ContainerResponseContext responseContext) {
        ScopeContext<Method> context = this.context;
        Method previous = this.previous;

        if (context == null ||
            previous == null) return;

        if (context.isActive()) {
            context.exit(previous);
        }
    }

    public @Nullable ResourceInfo getResourceInfoOrNull() {
        return this.resourceInfo;
    }

    public boolean isActive() {
        if (context == null) return false;
        return context.isActive();
    }
}
