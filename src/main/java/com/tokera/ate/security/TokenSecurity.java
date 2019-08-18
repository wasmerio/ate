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

        String alias = token.getClaimsForKey(TokenDto.SECURITY_CLAIM_USERNAME)
            .stream()
            .map(c -> c.getValue())
            .findFirst()
            .orElse(token.getClaimsForKey(TokenDto.SECURITY_CLAIM_USER_ID)
                    .stream()
                    .map(c -> "user://" + c.getValue())
                    .findFirst()
                    .orElse(null));

        this.writeRightsCache = new ImmutalizableHashSet<>();
        for (ClaimDto claimVal : token.getClaimsForKey(TokenDto.SECURITY_CLAIM_WRITE_KEY)) {
            String[] comps = claimVal.getValue().split(":");
            String keyAlias = comps.length > 1 ? comps[0] : alias;
            String keySeed = comps[comps.length-1];

            SigningKeyWithSeedDto keyWithSeed = new SigningKeyWithSeedDto(keySeed);
            MessagePrivateKeyDto newKey = new MessagePrivateKeyDto(keyWithSeed.key, keyAlias);
            this.writeRightsCache.add(newKey);
        }
        this.writeRightsCache.immutalize();

        this.readRightsCache = new ImmutalizableHashSet<>();
        for (ClaimDto claimVal : token.getClaimsForKey(TokenDto.SECURITY_CLAIM_READ_KEY)) {
            String[] comps = claimVal.getValue().split(":");
            String keyAlias = comps.length > 1 ? comps[0] : alias;
            String keySeed = comps[comps.length-1];

            EncryptKeyWithSeedDto keyWithSeed = new EncryptKeyWithSeedDto(keySeed);
            MessagePrivateKeyDto newKey = new MessagePrivateKeyDto(keyWithSeed.key, keyAlias);
            this.readRightsCache.add(newKey);
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
