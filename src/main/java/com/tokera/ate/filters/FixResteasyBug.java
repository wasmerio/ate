package com.tokera.ate.filters;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.scopes.Startup;

import java.io.IOException;
import javax.annotation.Priority;
import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import javax.servlet.http.HttpServletRequest;

import javax.ws.rs.container.ContainerRequestContext;
import javax.ws.rs.container.ContainerRequestFilter;
import javax.ws.rs.core.MediaType;
import javax.ws.rs.ext.Provider;

/**
 * This filter fixes a nasty bug with resteasy where parameter maps would not load properly for certina media types
 * @author jonhanlee
 */
@Startup
@ApplicationScoped
@Provider
@Priority(5020)
public class FixResteasyBug implements ContainerRequestFilter {

    protected AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private HttpServletRequest request;

    @Override
    public void filter(ContainerRequestContext requestContext) throws IOException {
        // Set the requestContext variable
        d.requestContext.setContainerRequestContext(requestContext);

        // If its a form then make sure the parameters are read before we attempt to process the currentRights stream or it will kill the mappings (Resteasy bug)
        if (requestContext.getMediaType() == MediaType.APPLICATION_FORM_URLENCODED_TYPE ||
            requestContext.getMediaType() == MediaType.MULTIPART_FORM_DATA_TYPE) {
            request.getParameterMap();
        }
    }
}
