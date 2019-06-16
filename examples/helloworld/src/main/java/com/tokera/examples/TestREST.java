package com.tokera.examples;

import com.tokera.ate.delegates.AteDelegate;

import javax.annotation.security.PermitAll;
import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.*;
import javax.ws.rs.core.MediaType;
import java.util.UUID;

@ApplicationScoped
@Path("/test")
public class TestREST {
    private AteDelegate d = AteDelegate.get();

    @POST
    @Path("add")
    @Consumes(MediaType.TEXT_PLAIN)
    @PermitAll
    public void addText(String val) {
        MyTextDao text = new MyTextDao();
        text.text = val;
        d.io.mergeLater(text);
    }

    @GET
    @Path("hello")
    @Produces(MediaType.TEXT_PLAIN)
    @PermitAll
    public String hellowWorld()
    {
        StringBuilder sb = new StringBuilder();
        sb.append("hi\n");

        for (MyTextDao text : d.io.getAll(MyTextDao.class)) {
            sb.append(text.text).append("\n");
        }

        return sb.toString();
    }
}
