package com.tokera.ate.dto;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.common.UUIDTools;
import com.tokera.ate.dao.enumerations.RiskRole;
import com.tokera.ate.dao.enumerations.UserRole;
import com.tokera.ate.units.*;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.opensaml.saml2.core.Assertion;
import org.opensaml.saml2.core.Attribute;
import org.opensaml.saml2.core.AttributeStatement;
import org.opensaml.saml2.core.impl.AssertionUnmarshaller;
import org.opensaml.xml.XMLObject;
import org.opensaml.xml.io.UnmarshallingException;
import org.opensaml.xml.parse.BasicParserPool;
import org.opensaml.xml.parse.XMLParserException;
import org.opensaml.xml.schema.XSString;
import org.w3c.dom.Document;

import javax.validation.constraints.NotNull;
import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;
import javax.ws.rs.core.Response.Status;
import java.io.ByteArrayInputStream;
import java.io.InputStream;
import java.io.UnsupportedEncodingException;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;
import javax.ws.rs.WebApplicationException;

/**
 * Represents an authentication and authorization token that has been signed by the issuer
 * @author John
 */
@YamlTag("dto.token")
public class TokenDto {

    @JsonProperty
    @NotNull
    @Size(min=1)
    private @TextDocument String xmlToken;
    @JsonProperty
    @Nullable
    @Size(min=43, max=43)
    @Pattern(regexp = "^(?:[A-Za-z0-9+\\/\\-_])*(?:[A-Za-z0-9+\\/\\-_]{2}==|[A-Za-z0-9+\\/\\-_]{3}=)?$")
    private @Hash String tokenHash;
    @JsonProperty
    @Nullable
    private List<ClaimDto> claimsCache = null;

    public static final String SECURITY_CLAIM_USERNAME = "claim://token/username";
    public static final String SECURITY_CLAIM_USER_ID = "claim://token/user-id";
    public static final String SECURITY_CLAIM_ACCOUNT_ID = "claim://token/account-id";
    public static final String SECURITY_CLAIM_NODE_ID = "claim://token/node-id";
    public static final String SECURITY_CLAIM_CLUSTER_ID = "claim://token/cluster-id";
    public static final String SECURITY_CLAIM_RISK_ROLE = "claim://token/risk-role";
    public static final String SECURITY_CLAIM_USER_ROLE = "claim://token/user-role";
    public static final String SECURITY_CLAIM_READ_KEY = "claim://token/read-key";
    public static final String SECURITY_CLAIM_WRITE_KEY = "claim://token/write-key";

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public TokenDto() {
    }

    public TokenDto(@TextDocument String xmlToken) {
        this.xmlToken = xmlToken;
    }

    /**
     * @return XML representation of the Token content
     */
    public @TextDocument String getXmlToken() {
        return this.xmlToken;
    }

    private static @Hash String computeTokenHash(@TextDocument String xmlToken) {
        try {
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            byte[] digestBytes = digest.digest(xmlToken.getBytes("UTF-8"));
            return Base64.encodeBase64String(digestBytes);
        } catch (NoSuchAlgorithmException | UnsupportedEncodingException ex) {
            throw new WebApplicationException("Unable to generate the token hash.", ex);
        }
    }

    /**
     * @return Hash of the Token which can be used as an caching index and/or as an authorization header in HTTP requests
     */
    public @Hash String getHash() {
        @Hash String ret = tokenHash;
        if (ret == null) {
            ret = computeTokenHash(this.xmlToken);
            tokenHash = ret;
        }
        return ret;
    }

    /**
     * @return Gets the signing assertion from the XML body of the token or throws an exception if none exists
     */
    public Assertion getAssertion() {
        // Convert the token to a assertion
        Assertion assertion;
        try {
            // Get parser pool manager
            BasicParserPool ppMgr = new BasicParserPool();
            ppMgr.setNamespaceAware(true);

            InputStream in = new ByteArrayInputStream(this.getXmlToken().getBytes());
            Document inCommonMDDoc = ppMgr.parse(in);
            AssertionUnmarshaller unmarshaller = new AssertionUnmarshaller();
            assertion = (Assertion) unmarshaller.unmarshall(inCommonMDDoc.getDocumentElement());
        } catch (UnmarshallingException | XMLParserException ex) {
            throw new WebApplicationException("Client passed an invalid Token", Status.BAD_REQUEST);
        }

        // return the assertion
        return assertion;
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

        List<ClaimDto> ret = claimsCache;
        if (ret != null) {
            return ret;
        }

        // Get the assertion
        Assertion assertion = getAssertion();

        // Get the claims
        ret = new ArrayList<>();
        for (AttributeStatement statement : assertion.getAttributeStatements()) {
            for (Attribute att : statement.getAttributes()) {
                if (att.getName().length() <= 0) {
                    continue;
                }

                for (XMLObject value : att.getAttributeValues()) {
                    if (value instanceof XSString) {
                        XSString valueStr = (XSString) value;
                        ClaimDto claim = new ClaimDto(
                                att.getName(),
                                valueStr.getValue());
                        ret.add(claim);
                    }
                }
            }
        }
        claimsCache = ret;
        return ret;
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
    public boolean hasClaim(@Alias String key, @Claim String value) {

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
}
