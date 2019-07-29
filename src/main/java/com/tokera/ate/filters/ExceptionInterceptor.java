package com.tokera.ate.filters;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.io.repo.DataTransaction;

import javax.annotation.Priority;
import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.Response;
import javax.ws.rs.ext.ExceptionMapper;
import javax.ws.rs.ext.Provider;
import java.io.PrintWriter;
import java.io.StringWriter;

/**
 * Controls the way API exceptions are logged *
 * @author jonhanlee
 */
@ApplicationScoped
@Provider
@Priority(5210)
public class ExceptionInterceptor implements ExceptionMapper<Throwable> {

    protected AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;

    public static boolean g_extraLogging = false;
    public static boolean g_includeStack = true;

    @Override
    public Response toResponse(Throwable ex) {
        
        try {
            d.io.clearAll();
        } catch (Throwable ex1) {
            this.LOG.warn(ex1);
        }
        
        if (ex instanceof WebApplicationException) {
            WebApplicationException exception = (WebApplicationException) ex;
            String msg = exception.getMessage();
            if (msg == null) msg = exception.getClass().getSimpleName();
            if (ExceptionInterceptor.g_extraLogging) {
                this.LOG.error(ex);
            }
            if (ExceptionInterceptor.g_includeStack) {
                StringWriter sw = new StringWriter();
                ex.printStackTrace(new PrintWriter(sw));
                msg = sw.toString();
            }
            return Response.status(exception.getResponse().getStatus()).entity(msg).build();
        } else {
            String msg = ex.getMessage();
            if (msg == null) msg = ex.getClass().getSimpleName();
            if (ExceptionInterceptor.g_extraLogging) {
                this.LOG.error(ex);
            }
            if (ExceptionInterceptor.g_includeStack) {
                StringWriter sw = new StringWriter();
                ex.printStackTrace(new PrintWriter(sw));
                msg = sw.toString();
            }
            return Response.status(Response.Status.INTERNAL_SERVER_ERROR).entity(msg).build();
        }
    }
}
