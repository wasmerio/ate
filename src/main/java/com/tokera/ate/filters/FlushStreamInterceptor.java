package com.tokera.ate.filters;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.events.TokenScopeChangedEvent;
import com.tokera.ate.io.api.IPartitionKey;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.annotation.Priority;
import javax.enterprise.context.RequestScoped;
import javax.enterprise.event.Observes;
import javax.inject.Inject;
import javax.servlet.http.HttpServletRequest;
import javax.servlet.http.HttpServletResponse;
import javax.ws.rs.Priorities;
import javax.ws.rs.container.ContainerRequestContext;
import javax.ws.rs.container.ContainerRequestFilter;
import javax.ws.rs.container.ContainerResponseContext;
import javax.ws.rs.container.ContainerResponseFilter;
import javax.ws.rs.core.Context;
import javax.ws.rs.core.Cookie;
import javax.ws.rs.ext.Provider;
import java.io.IOException;
import java.io.OutputStream;
import java.util.Map;

/**
 * Filter that will read and process tokens that are passed to the API REST calls
 */
@RequestScoped
@Provider
@Priority(Priorities.USER)
public class FlushStreamInterceptor implements ContainerResponseFilter {
    protected AteDelegate d = AteDelegate.get();

    @Override
    public void filter(ContainerRequestContext requestContext, ContainerResponseContext responseContext) throws IOException {
        if (responseContext.getStatus() >= 200 && responseContext.getStatus() < 400) {
            OutputStream stream = d.logging.getRedirectStream();
            if (stream != null) {
                stream.flush();
                stream.close();
                d.logging.redirect(null);
            }
        }
    }
}
