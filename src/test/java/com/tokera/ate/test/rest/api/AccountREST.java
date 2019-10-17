package com.tokera.ate.test.rest.api;

import com.tokera.ate.annotations.PermitRiskRole;
import com.tokera.ate.annotations.PermitUserRole;
import com.tokera.ate.common.StringTools;
import com.tokera.ate.common.UUIDTools;
import com.tokera.ate.dao.enumerations.RiskRole;
import com.tokera.ate.dao.enumerations.UserRole;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.security.TokenBuilder;
import com.tokera.ate.test.dao.MyAccount;
import com.tokera.ate.test.dto.NewAccountDto;
import com.tokera.ate.units.EmailAddress;
import org.junit.jupiter.api.Assertions;

import javax.annotation.security.PermitAll;
import javax.enterprise.context.ApplicationScoped;
import javax.mail.MessagingException;
import javax.validation.Valid;
import javax.ws.rs.*;
import javax.ws.rs.core.MediaType;
import java.io.IOException;
import java.util.Random;
import java.util.UUID;

@ApplicationScoped
@Path("/acc")
public class AccountREST {
    protected final AteDelegate d = AteDelegate.get();
    private final Random rand = new Random();

    @POST
    @Path("adminToken/{username}")
    @Produces(MediaType.APPLICATION_XML)
    @Consumes({"text/yaml", MediaType.APPLICATION_JSON})
    @PermitAll
    public String createAdminToken(@PathParam("username") String username, @Valid PrivateKeyWithSeedDto key)
    {
        // Set the alias in the key to be the username
        username = username + "@mycompany.org";

        PrivateKeyWithSeedDto anotherKey = d.encryptor.genSignKeyAndSeed();
        anotherKey.setAlias("useless-key@nowhere.com");

        return new TokenBuilder()
                .withUsername(username)
                .withRiskRole(RiskRole.HIGH)
                .withUserRole(UserRole.HUMAN)
                .addWriteKey(key)
                .addWriteKey(anotherKey)
                .shouldPublish(true)
                .build()
                .getBase64();
    }

    @PUT
    @Path("register")
    @Consumes(MediaType.APPLICATION_JSON)
    @Produces(MediaType.APPLICATION_JSON)
    @PermitUserRole(UserRole.HUMAN)
    @PermitRiskRole(RiskRole.HIGH)
    public MyAccount registerAccount(@Valid NewAccountDto theDetails) throws IOException, MessagingException
    {
        @EmailAddress String email = theDetails.getEmail();
        assert email != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(email);

        MyAccount acc = new MyAccount(email, "pass123");
        acc.id = UUIDTools.generateUUID(StringTools.getDomain(email));
        acc.description = theDetails.getDescription();
        d.authorization.authorizeEntityPublicRead(acc);
        d.authorization.authorizeEntityPublicWrite(acc);
        d.io.write(acc);
        return acc;
    }

    @GET
    @Path("/{id}")
    @Produces(MediaType.APPLICATION_JSON)
    @PermitUserRole(UserRole.HUMAN)
    @PermitRiskRole(RiskRole.MEDIUM)
    public MyAccount getAccount(@PathParam("id") UUID id) {
        return d.io.read(id, MyAccount.class);
    }

    @GET
    @Path("/{id}/touch")
    @Produces(MediaType.APPLICATION_JSON)
    @PermitUserRole(UserRole.HUMAN)
    @PermitRiskRole(RiskRole.MEDIUM)
    public MyAccount touchAccount(@PathParam("id") UUID id) {
        MyAccount ret = d.io.read(id, MyAccount.class);
        ret.counter.increment();
        d.io.write(ret);
        return ret;
    }

    @POST
    @Path("/{id}/addThing")
    @Consumes(MediaType.APPLICATION_JSON)
    @Produces(MediaType.APPLICATION_JSON)
    @PermitUserRole(UserRole.HUMAN)
    @PermitRiskRole(RiskRole.MEDIUM)
    public MyAccount addThing(@PathParam("id") UUID id, UUID val) throws InterruptedException {
        MyAccount ret = d.io.read(id, MyAccount.class);
        ret.things.add(val);
        //Thread.sleep(5 + rand.nextInt(5));
        d.io.write(ret);
        return ret;
    }
}
