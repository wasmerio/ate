package com.tokera.examples.rest;

import com.tokera.ate.dao.enumerations.KeyType;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.enumerations.PrivateKeyType;
import com.tokera.ate.security.TokenBuilder;
import com.tokera.examples.dto.PasswordLoginRequest;
import com.tokera.examples.dto.RootLoginRequest;

import javax.annotation.security.PermitAll;
import javax.enterprise.context.RequestScoped;
import javax.ws.rs.Consumes;
import javax.ws.rs.POST;
import javax.ws.rs.Path;
import javax.ws.rs.Produces;
import javax.ws.rs.core.MediaType;

@RequestScoped
@Path("/login")
public class LoginREST {
    protected AteDelegate d = AteDelegate.get();

    @POST
    @Path("/root")
    @Produces(MediaType.APPLICATION_XML)
    @Consumes({"text/yaml", MediaType.APPLICATION_JSON})
    @PermitAll
    public String rootLogin(RootLoginRequest request) {
        return new TokenBuilder()
                .withUsername(request.getUsername())
                .addReadKeys(request.getReadRights())
                .addWriteKeys(request.getWriteRights())
                .shouldPublish(true)
                .build()
                .getBase64();
    }

    @POST
    @Path("/password")
    @Produces(MediaType.APPLICATION_XML)
    @Consumes({"text/yaml", MediaType.APPLICATION_JSON})
    @PermitAll
    public String passwordLogin(PasswordLoginRequest request) {
        PrivateKeyWithSeedDto writeKey = new PrivateKeyWithSeedDto(PrivateKeyType.write, request.getPasswordHash(), 256, KeyType.qtesla, null, request.getUsername());
        PrivateKeyWithSeedDto readKey = new PrivateKeyWithSeedDto(PrivateKeyType.read, request.getPasswordHash(), 256, KeyType.ntru, null, request.getUsername());
        return new TokenBuilder()
                .withUsername(request.getUsername())
                .addReadKey(readKey)
                .addWriteKey(writeKey)
                .shouldPublish(true)
                .build()
                .getBase64();
    }

    @POST
    @Path("/token")
    @Produces(MediaType.TEXT_PLAIN)
    @Consumes(MediaType.APPLICATION_XML)
    @PermitAll
    public String tokenLogin(String tokenTxt) {
        TokenDto token = new TokenDto(tokenTxt);
        d.currentToken.publishToken(token);
        return token.getBase64();
    }
}
