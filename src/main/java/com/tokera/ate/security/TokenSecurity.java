package com.tokera.ate.security;

import com.tokera.ate.common.ImmutalizableHashSet;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.ClaimDto;
import com.tokera.ate.dto.EncryptKeyWithSeedDto;
import com.tokera.ate.dto.SigningKeyWithSeedDto;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.scopes.TokenScoped;
import com.tokera.ate.units.*;

import javax.inject.Inject;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentMap;

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
        TokenDto token = new TokenDto(d.currentToken.getTokenScopeValue());
        if (token == null) d.currentToken.missingToken();
        token.validate();

        this.token = token;

        this.writeRightsCache = new ImmutalizableHashSet<>();
        for (ClaimDto keySeed : token.getClaimsForKey(TokenDto.SECURITY_CLAIM_WRITE_KEY)) {
            SigningKeyWithSeedDto key = new SigningKeyWithSeedDto(keySeed.getValue());
            this.writeRightsCache.add(key.key);
        }
        this.writeRightsCache.immutalize();

        this.readRightsCache = new ImmutalizableHashSet<>();
        for (ClaimDto keySeed : token.getClaimsForKey(TokenDto.SECURITY_CLAIM_READ_KEY)) {
            EncryptKeyWithSeedDto key = new EncryptKeyWithSeedDto(keySeed.getValue());
            this.readRightsCache.add(key.key);
        }
        this.readRightsCache.immutalize();
    }
    
    public Map<String, byte[]> getEncryptKeyCache() {
        return this.encryptKeyCache;
    }

    /**
     * Generates a SAML2 token, compresses it and encodes it in Base64.
     */
    public static TokenDto generateToken(Map<@Alias String, List<String>> claims, int expiresMins) {
        return new TokenDto(claims, expiresMins);
    }

    public static void addClaim(Map<@Alias String, List<String>> map, @Alias String key, String value) {
        if (!map.containsKey(key)) {
            map.put(key, new ArrayList<>());
        }
        List<String> keyValues = map.get(key);
        keyValues.add(value);
    }

    public static void clearClaims(Map<@Alias String, List<String>> map, @Alias String key) {
        map.remove(key);
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
