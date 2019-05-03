package com.tokera.ate.test.rest.api;

import com.tokera.ate.annotations.PermitRiskRole;
import com.tokera.ate.annotations.PermitUserRole;
import com.tokera.ate.common.StringTools;
import com.tokera.ate.common.UUIDTools;
import com.tokera.ate.configuration.AteConstants;
import com.tokera.ate.dao.enumerations.RiskRole;
import com.tokera.ate.dao.enumerations.UserRole;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.security.TokenSecurity;
import com.tokera.ate.test.dao.MyAccount;
import com.tokera.ate.test.dao.SeedingDelegate;
import com.tokera.ate.test.dto.NewAccountDto;
import com.tokera.ate.units.Claim;
import com.tokera.ate.units.DomainName;
import com.tokera.ate.units.EmailAddress;
import org.junit.jupiter.api.Assertions;

import javax.annotation.security.PermitAll;
import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.inject.spi.CDI;
import javax.mail.MessagingException;
import javax.validation.Valid;
import javax.ws.rs.*;
import javax.ws.rs.core.MediaType;
import java.io.IOException;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;

@ApplicationScoped
@Path("/acc")
public class AccountREST {

    protected AteDelegate d = AteDelegate.get();

    @POST
    @Path("adminToken/{username}")
    @Produces(MediaType.APPLICATION_XML)
    @Consumes({"text/yaml", MediaType.APPLICATION_JSON})
    @PermitAll
    public String createAdminToken(@PathParam("username") String username, @Valid MessagePrivateKeyDto key) {
        // Set the alias in the key to be the username
        username = username + "@mycompany.org";
        key = CDI.current().select(SeedingDelegate.class).get().getRootKey();

        // Create the claims
        Map<String, List<@Claim String>> claims = new HashMap<>();
        TokenSecurity.addClaim(claims, TokenDto.SECURITY_CLAIM_USERNAME, username);
        TokenSecurity.addClaim(claims, TokenDto.SECURITY_CLAIM_USER_ID, UUIDTools.generateUUID(username).toString());
        TokenSecurity.addClaim(claims, TokenDto.SECURITY_CLAIM_RISK_ROLE, RiskRole.HIGH.name());
        TokenSecurity.addClaim(claims, TokenDto.SECURITY_CLAIM_RISK_ROLE, RiskRole.MEDIUM.name());
        TokenSecurity.addClaim(claims, TokenDto.SECURITY_CLAIM_RISK_ROLE, RiskRole.LOW.name());
        TokenSecurity.addClaim(claims, TokenDto.SECURITY_CLAIM_RISK_ROLE, RiskRole.NONE.name());
        TokenSecurity.addClaim(claims, TokenDto.SECURITY_CLAIM_USER_ROLE, UserRole.HUMAN.name());
        TokenSecurity.addClaim(claims, TokenDto.SECURITY_CLAIM_WRITE_KEY, d.yaml.serializeObj(key));

        // Generate the token and return it
        @DomainName String domain = StringTools.getDomain(username);
        TokenDto token = TokenSecurity.generateToken("mycompany.org", domain, username, domain, claims, 100);
        d.currentToken.publishToken(token);

        return token.getXmlToken();
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
        d.headIO.mergeLater(acc);
        return acc;
    }
}
