package com.tokera.ate.security;

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
 */
@TokenScoped
public class TokenSecurity
{
    private AteDelegate d = AteDelegate.getUnsafe();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    protected LoggerHook LOG;

    @SuppressWarnings("initialization.fields.uninitialized")
    private BasicX509Credential signingCredential;
    @SuppressWarnings("initialization.fields.uninitialized")
    private SignatureValidator validator;
    
    private final ConcurrentMap<String, byte[]> encryptKeyCache = new ConcurrentHashMap<>();

    @MonotonicNonNull
    private TokenDto token;
    
    // List of all the rights the token bearer has to read objects
    @MonotonicNonNull
    private Set<MessagePrivateKeyDto> readRightsCache;
    // List of all the rights the token bearer has to write objects
    @MonotonicNonNull
    private Set<MessagePrivateKeyDto> writeRightsCache;

    @PostConstruct
    public void init() {
        
        // Load the certificate
        signingCredential = SignAssertion.getSigningCredentialCached();
        validator = new SignatureValidator(signingCredential);
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
            validator.validate(assertion.getSignature());
        } catch (ValidationException e) {
            throw new WebApplicationException("Token signature is not valid", e, Response.Status.UNAUTHORIZED);
        }
    }

    public void setToken(TokenDto token) {
        validateToken(token);
        this.token = token;
    }

    /**
     * @return the token
     */
    public TokenDto getToken() {
        if (this.token == null) {
            throw new WebApplicationException("There is not token currentRights attached to this token scope.");
        }
        return this.token;
    }

    public @Nullable TokenDto getTokenOrNull() {
        return this.token;
    }
    
    public Set<MessagePrivateKeyDto> getRightsRead() {
        if (readRightsCache != null) {
            return readRightsCache;
        }
        
        Set<MessagePrivateKeyDto> ret = new HashSet<>();
        
        if (token == null) return new HashSet<>();
        
        for (ClaimDto key : token.getClaimsForKey(TokenDto.SECURITY_CLAIM_READ_KEY)) {
            ret.add((MessagePrivateKeyDto)d.yaml.deserializeObj(key.getValue()));
        }
        
        readRightsCache = ret.stream().collect(Collectors.toSet());
        return readRightsCache;
    }

    public Set<MessagePrivateKeyDto> getRightsWrite() {
        if (writeRightsCache != null) {
            return writeRightsCache;
        }
        
        Set<MessagePrivateKeyDto> ret = new HashSet<>();
        
        if (token == null) return new HashSet<>();
        
        for (ClaimDto key : token.getClaimsForKey(TokenDto.SECURITY_CLAIM_WRITE_KEY)) {
            ret.add((MessagePrivateKeyDto)d.yaml.deserializeObj(key.getValue()));
        }
        
        writeRightsCache = ret.stream().collect(Collectors.toSet());
        return writeRightsCache;
    }
}
