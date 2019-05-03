package com.tokera.ate.test.rest.api;

import com.tokera.ate.delegates.AteDelegate;

import javax.annotation.security.PermitAll;
import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.*;
import javax.ws.rs.core.MediaType;
import java.util.UUID;

@ApplicationScoped
@Path("/test")
public class TestREST {

    protected AteDelegate d = AteDelegate.get();

    @GET
    @Path("uuid")
    @Produces(MediaType.TEXT_PLAIN)
    @PermitAll
    public UUID testUuidSerializer() {
        return UUID.randomUUID();
    }
}
