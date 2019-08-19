package com.tokera.ate.dto;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.databind.annotation.JsonDeserialize;
import com.fasterxml.jackson.databind.annotation.JsonSerialize;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.common.ImmutalizableArrayList;
import com.tokera.ate.common.UUIDTools;
import com.tokera.ate.dao.enumerations.RiskRole;
import com.tokera.ate.dao.enumerations.UserRole;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.providers.PartitionKeySerializer;
import com.tokera.ate.providers.TokenJsonDeserializer;
import com.tokera.ate.providers.TokenJsonSerializer;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.EmailAddress;
import com.tokera.ate.units.TextDocument;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.validation.constraints.NotNull;
import javax.validation.constraints.Size;
import javax.ws.rs.WebApplicationException;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.atomic.AtomicBoolean;

/**
 * Represents an authentication and authorization token that has been signed by the issuer
 * @author John
 * Note: This class must be multiple safe
 */
@YamlTag("token")
@JsonSerialize(using = TokenJsonSerializer.class)
@JsonDeserialize(using = TokenJsonDeserializer.class)
public class TokenDto {

    @JsonProperty
    @NotNull
    @Size(min=1)
    private @TextDocument String base64;
    @JsonProperty
    @Nullable
    private ImmutalizableArrayList<ClaimDto> claimsCache = null;
    @JsonIgnore
    private transient AtomicBoolean validated = new AtomicBoolean(false);

    public static final String SECURITY_CLAIM_USERNAME = "usr";
    public static final String SECURITY_CLAIM_USER_ID = "uid";
    public static final String SECURITY_CLAIM_ACCOUNT_ID = "aid";
    public static final String SECURITY_CLAIM_NODE_ID = "nid";
    public static final String SECURITY_CLAIM_CLUSTER_ID = "cid";
    public static final String SECURITY_CLAIM_RISK_ROLE = "rsk";
    public static final String SECURITY_CLAIM_USER_ROLE = "rol";
    public static final String SECURITY_CLAIM_READ_KEY = "rd";
    public static final String SECURITY_CLAIM_WRITE_KEY = "wrt";
    public static final String SECURITY_CLAIM_PARTITION_KEY = "key";

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public TokenDto() {
    }

    public TokenDto(@TextDocument String base64) {
        this.base64 = base64;
    }

    public TokenDto(Map<@Alias String, List<String>> claims, int expiresMins) {
        this.base64 = AteDelegate.get().authorization.createToken(claims, expiresMins);
    }

    /**
     * Validates the token and throws an exception if its not validate
     */
    public void validate() {
        if (this.validated.get() == true) return;
        AteDelegate.get().authorization.validateToken(this.base64);
        this.validated.set(true);
    }

    /**
     * @return List of claims within the Token that match a particular key (claims are key/value pairs)
     */
    public List<ClaimDto> getClaimsForKey(@Alias String key) {
        List<ClaimDto> ret = new ArrayList<>();
        for (ClaimDto claim : getClaims()) {
            if (key.equals(claim.getKey()) == true) {
                ret.add(claim);
            }
        }
        return ret;
    }

    /**
     * @return All the claims that are contained within the Token
     */
    public List<ClaimDto> getClaims() {

        ImmutalizableArrayList<ClaimDto> ret = claimsCache;
        if (ret != null) {
            return ret;
        }

        synchronized (this)
        {
            ret = claimsCache;
            if (ret != null) {
                return ret;
            }

            ret = AteDelegate.get().authorization.extractTokenClaims(this.base64);

            claimsCache = ret;
            return ret;
        }
    }

    /**
     * @return True if a particular risk role type is claimed within this token
     */
    public boolean hasRiskRole(RiskRole role) {
        return hasClaim(TokenDto.SECURITY_CLAIM_RISK_ROLE, role.name());
    }

    /**
     * @return True if a particular user role type is claimed within this token
     */
    public boolean hasUserRole(UserRole role) {
        if (hasClaim(TokenDto.SECURITY_CLAIM_USER_ROLE, UserRole.ANYTHING.name())) return true;
        return hasClaim(TokenDto.SECURITY_CLAIM_USER_ROLE, role.name());
    }

    /**
     * @return True if this token contains a claim for the currentRights user ID
     */
    public boolean hasUserId() {
        for (ClaimDto claim : getClaims()) {
            if (claim.getKey().equalsIgnoreCase(TokenDto.SECURITY_CLAIM_USER_ID)) {
                @DaoId UUID ret = UUIDTools.parseUUIDorNull(claim.getValue());
                if (ret != null) return true;
            }
        }
        return false;
    }

    /**
     * @return User ID contains within this Token or throws an exception
     */
    public @DaoId UUID getUserId() {
        for (ClaimDto claim : getClaims()) {
            if (claim.getKey().equalsIgnoreCase(TokenDto.SECURITY_CLAIM_USER_ID)) {
                @DaoId UUID ret = UUIDTools.parseUUIDorNull(claim.getValue());
                if (ret != null) return ret;
            }
        }
        throw new WebApplicationException("Unable to find user ID in token.");
    }

    /**
     * @return User ID contains within this Token or throws an exception
     */
    public @Nullable @DaoId UUID getUserIdOrNull() {
        for (ClaimDto claim : getClaims()) {
            if (claim.getKey().equalsIgnoreCase(TokenDto.SECURITY_CLAIM_USER_ID)) {
                @DaoId UUID ret = UUIDTools.parseUUIDorNull(claim.getValue());
                if (ret != null) return ret;
            }
        }
        return null;
    }

    /**
     * @return Email address of the user within this Token or throws an exception
     */
    public @EmailAddress String getUsername() {
        for (ClaimDto claim : getClaims()) {
            if (claim.getKey().equalsIgnoreCase(TokenDto.SECURITY_CLAIM_USERNAME)) {
                return claim.getValue();
            }
        }
        throw new WebApplicationException("Unable to find username in token.");
    }

    /**
     * @return Partition key that is associated with this token (if any)
     */
    public @Nullable IPartitionKey getPartitionKeyOrNull() {
        for (ClaimDto claim : getClaims()) {
            if (claim.getKey().equalsIgnoreCase(TokenDto.SECURITY_CLAIM_PARTITION_KEY)) {
                PartitionKeySerializer serializer = new PartitionKeySerializer();
                return serializer.read(claim.getValue());
            }
        }
        return null;
    }

    /**
     * @return First UUID claim within the Token that matches a particular key or null if it doesnt exist
     */
    public @Nullable @DaoId UUID getIdOrNull(String key) {
        for (ClaimDto claim : this.getClaims()) {
            if (claim.getKey().equals(key)) {
                return UUIDTools.parseUUIDorNull(claim.getValue());
            }
        }
        return null;
    }

    /**
     * @return True if a particular claim exists that matches both the key and the value
     */
    public boolean hasClaim(@Alias String key, String value) {

        for (ClaimDto claim : getClaims()) {
            if (claim.getKey().compareToIgnoreCase(key) != 0) {
                continue;
            }
            if (claim.getValue().compareToIgnoreCase(value) != 0) {
                continue;
            }
            return true;
        }
        return false;
    }

    /**
     * Returns a Base64 representation of the token
     */
    public String getBase64() {
        return this.base64;
    }

    /**
     * Sets the validated flag
     */
    public void setValidated(boolean val) {
        this.validated.set(val);
    }

    @Override
    public String toString() {
        return this.base64;
    }

    @Override
    public int hashCode() {
        return this.base64.hashCode();
    }

    @Override
    public boolean equals(Object other) {
        if (other == null) return false;
        if (other instanceof TokenDto) {
            TokenDto otherToken = (TokenDto)other;
            if (this.base64.equals(otherToken.base64) == false) return false;
            return true;
        }
        return false;
    }
}
