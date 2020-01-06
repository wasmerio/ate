package com.tokera.ate.test.rest.api;

import com.tokera.ate.delegates.AteDelegate;
import org.eclipse.microprofile.faulttolerance.Timeout;

import javax.annotation.security.PermitAll;
import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.*;
import javax.ws.rs.core.MediaType;
import java.util.UUID;

@ApplicationScoped
@Path("/test")
@Timeout
public class TestREST {
    protected AteDelegate d = AteDelegate.get();

    @GET
    @Path("uuid")
    @Produces(MediaType.TEXT_PLAIN)
    @PermitAll
    public UUID testUuidSerializer() {
        return UUID.randomUUID();
    }

    @GET
    @PermitAll
    @Timeout(100)
    @Path("timeout")
    public String shouldTimeout() throws InterruptedException {
        Thread.sleep(1000);
        return "not-me";
    }

    @GET
    @PermitAll
    @Timeout(100)
    @Path("no-timeout")
    public String shouldNotTimeout() throws InterruptedException {
        return "not-me";
    }
}
