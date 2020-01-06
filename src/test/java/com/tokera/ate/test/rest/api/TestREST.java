package com.tokera.ate.test.rest.api;

import com.tokera.ate.delegates.AteDelegate;
import org.eclipse.microprofile.faulttolerance.Timeout;

import javax.annotation.security.PermitAll;
import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import javax.ws.rs.*;
import javax.ws.rs.core.MediaType;
import java.util.UUID;

@ApplicationScoped
@Path("/test")
@Timeout(20000)
public class TestREST {
    protected AteDelegate d = AteDelegate.get();

    @Inject
    private RequestWork requestWork;

    @GET
    @Path("custom-data")
    @Produces(MediaType.TEXT_PLAIN)
    @PermitAll
    public Object testCustomData() {
        return d.requestContext.getCustomData();
    }

    @GET
    @Path("uuid")
    @Produces(MediaType.TEXT_PLAIN)
    @PermitAll
    public UUID testUuidSerializer() {
        requestWork.doWork();
        return UUID.randomUUID();
    }

    @GET
    @Path("ping")
    @Produces(MediaType.TEXT_PLAIN)
    @PermitAll
    public String ping() {
        return "pong";
    }

    @GET
    @PermitAll
    @Timeout(100)
    @Path("timeout")
    public String shouldTimeout() throws InterruptedException {
        Thread.sleep(1000);
        requestWork.doWork();
        return "not-me";
    }

    @GET
    @PermitAll
    @Timeout(100)
    @Path("no-timeout")
    public String shouldNotTimeout() throws InterruptedException {
        requestWork.doWork();
        return "not-me";
    }
}
