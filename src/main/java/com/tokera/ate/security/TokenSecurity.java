package com.tokera.ate.security;

import com.tokera.ate.common.ImmutalizableHashSet;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.token.SAMLWriter;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.dto.ClaimDto;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.scopes.TokenScoped;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.token.SignAssertion;
import com.tokera.ate.units.*;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import javax.inject.Inject;
import java.util.HashSet;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentMap;
import java.util.stream.Collectors;
import javax.annotation.PostConstruct;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.Response;

import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.opensaml.saml2.core.Assertion;
import org.opensaml.xml.security.x509.BasicX509Credential;
import org.opensaml.xml.signature.SignatureValidator;
import org.opensaml.xml.validation.ValidationException;

/**
 * Represents the Token loaded into a token scope
 * NOTE: This delegate must be multithread safe
 */
@TokenScoped
public class TokenSecurity
{
    private AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    protected LoggerHook LOG;

    private final ConcurrentMap<String, byte[]> encryptKeyCache = new ConcurrentHashMap<>();
    private final TokenDto token;
    private final ImmutalizableHashSet<MessagePrivateKeyDto> readRightsCache;
    private final ImmutalizableHashSet<MessagePrivateKeyDto> writeRightsCache;

    public TokenSecurity()
    {
        TokenDto token = d.currentToken.getInitTokenOrNull();
        if (token == null) d.currentToken.missingToken();

        if (token.validated.compareAndSet(false, true)) {
            validateToken(token);
        }

        this.token = token;

        this.writeRightsCache = new ImmutalizableHashSet<>();
        for (ClaimDto key : token.getClaimsForKey(TokenDto.SECURITY_CLAIM_WRITE_KEY)) {
            this.writeRightsCache.add((MessagePrivateKeyDto)d.yaml.deserializeObj(key.getValue()));
        }
        this.writeRightsCache.immutalize();

        this.readRightsCache = new ImmutalizableHashSet<>();
        for (ClaimDto key : token.getClaimsForKey(TokenDto.SECURITY_CLAIM_READ_KEY)) {
            this.readRightsCache.add((MessagePrivateKeyDto)d.yaml.deserializeObj(key.getValue()));
        }
        this.readRightsCache.immutalize();
    }
    
    public Map<String, byte[]> getEncryptKeyCache() {
        return this.encryptKeyCache;
    }

    /**
     * Generates a SAML2 token, compresses it and encodes it in Base64.
     */
    public static TokenDto generateToken(String company, @ReferenceNumber String reference, @EmailAddress String username, @PlainText String nameQualifier, Map<@Alias String, List<@Claim String>> claims, int expiresMins) {
        return SAMLWriter.createToken(company, reference, username, nameQualifier, claims, expiresMins);
    }

    public static void addClaim(Map<@Alias String, List<@Claim String>> map, @Alias String key, @Claim String value) {
        if (!map.containsKey(key)) {
            map.put(key, new ArrayList<>());
        }
        List<@Claim String> keyValues = map.get(key);
        keyValues.add(value);
    }
    
    /**
     * Validates the contents of a token
     * @param token 
     */
    public void validateToken(TokenDto token)
    {
        // Convert the token to a assertion
        Assertion assertion = token.getAssertion();

        // Validate the assertion
        try {
            BasicX509Credential signingCredential = SignAssertion.getSigningCredential();
            SignatureValidator validator = new SignatureValidator(signingCredential);
            
            validator.validate(assertion.getSignature());
        } catch (ValidationException e) {
            throw new WebApplicationException("Token signature is not valid", e, Response.Status.UNAUTHORIZED);
        }
    }

    public TokenDto getToken() {
        return this.token;
    }
    
    public Set<MessagePrivateKeyDto> getRightsRead() {
        return readRightsCache;
    }

    public Set<MessagePrivateKeyDto> getRightsWrite() {
        return writeRightsCache;
    }
}
