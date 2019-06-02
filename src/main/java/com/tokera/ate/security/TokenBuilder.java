package com.tokera.ate.security;

import com.tokera.ate.common.StringTools;
import com.tokera.ate.common.UUIDTools;
import com.tokera.ate.dao.IRights;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.enumerations.RiskRole;
import com.tokera.ate.dao.enumerations.UserRole;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.providers.PartitionKeySerializer;
import com.tokera.ate.units.DomainName;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.UUID;

public class TokenBuilder {
    @Nullable
    private String company;
    @Nullable
    private String reference;
    @Nullable
    private String username;
    @Nullable
    private String nameQualifier;
    private final Map<String, List<String>> claims = new TreeMap<>();
    private int expiresMins = 0;
    private boolean partitionKeySet = false;
    private boolean riskRoleSet = false;
    private boolean userRoleSet = false;
    private boolean shouldPublish = false;

    public TokenBuilder() {
    }

    public TokenBuilder withCompanyName(String companyName) {
        this.company = companyName;
        return this;
    }

    public TokenBuilder withReference(String reference) {
        this.reference = reference;
        return this;
    }

    public TokenBuilder withUsername(String username) {
        this.username = username;
        return this;
    }

    public TokenBuilder withNameQualifier(String nameQualifier) {
        this.nameQualifier = nameQualifier;
        return this;
    }

    public TokenBuilder withExpiresMins(int expiresMins) {
        this.expiresMins = expiresMins;
        return this;
    }

    public TokenBuilder withRiskRole(RiskRole role) {
        if (riskRoleSet == true) {
            throw new RuntimeException("The risk role was already set earlier in the builder.");
        }
        riskRoleSet = true;

        RiskRole rollingRole = role;
        for (;;) {
            TokenSecurity.addClaim(this.claims, TokenDto.SECURITY_CLAIM_RISK_ROLE, rollingRole.name());
            switch (rollingRole) {
                case HIGH:
                    rollingRole = RiskRole.MEDIUM;
                    continue;
                case MEDIUM:
                    rollingRole = RiskRole.LOW;
                    continue;
                default:
                    break;
            }
            break;
        }
        return this;
    }

    public TokenBuilder withUserRole(UserRole role) {
        if (userRoleSet == true) {
            throw new RuntimeException("The risk role was already set earlier in the builder.");
        }
        userRoleSet = true;

        TokenSecurity.addClaim(this.claims, TokenDto.SECURITY_CLAIM_USER_ROLE, role.name());
        return this;
    }

    public TokenBuilder withPartitionKeyFromRights(IRights rights) {
        IPartitionKey key = AteDelegate.get().headIO.partitionResolver().resolve(rights);
        return withPartitionkey(key);
    }

    public TokenBuilder withPartitionkeyFromDao(BaseDao obj) {
        IPartitionKey key = AteDelegate.get().headIO.partitionResolver().resolve(obj);
        return withPartitionkey(key);
    }

    public TokenBuilder withPartitionkey(IPartitionKey key) {
        if (partitionKeySet == true) {
            throw new RuntimeException("The partition key was already set earlier in the builder.");
        }
        partitionKeySet = true;

        PartitionKeySerializer serializer = new PartitionKeySerializer();
        String keyTxt = serializer.write(key);

        TokenSecurity.addClaim(this.claims, TokenDto.SECURITY_CLAIM_PARTITION_KEY, keyTxt);
        return this;
    }

    public TokenBuilder addClaim(String key, String value) {
        TokenSecurity.addClaim(this.claims, key, value);
        return this;
    }

    public TokenBuilder addReadKey(MessagePrivateKeyDto key) {
        AteDelegate d = AteDelegate.get();
        TokenSecurity.addClaim(this.claims, TokenDto.SECURITY_CLAIM_READ_KEY, d.yaml.serializeObj(key));
        return this;
    }

    public TokenBuilder addReadKeys(Iterable<MessagePrivateKeyDto> keys) {
        for (MessagePrivateKeyDto key : keys) {
            addReadKey(key);
        }
        return this;
    }

    public TokenBuilder addWriteKey(MessagePrivateKeyDto key) {
        AteDelegate d = AteDelegate.get();
        TokenSecurity.addClaim(this.claims, TokenDto.SECURITY_CLAIM_WRITE_KEY, d.yaml.serializeObj(key));
        return this;
    }

    public TokenBuilder addWriteKeys(Iterable<MessagePrivateKeyDto> keys) {
        for (MessagePrivateKeyDto key : keys) {
            addWriteKey(key);
        }
        return this;
    }

    public TokenBuilder shouldPublish(boolean shouldPublish) {
        this.shouldPublish = shouldPublish;
        return this;
    }

    private void reconcileClaims() {
        AteDelegate d = AteDelegate.get();
        if (this.username != null) {
            if (this.claims.containsKey(TokenDto.SECURITY_CLAIM_USERNAME) == false) {
                TokenSecurity.addClaim(this.claims, TokenDto.SECURITY_CLAIM_USERNAME, this.username);
            }
            if (this.claims.containsKey(TokenDto.SECURITY_CLAIM_USER_ID) == false) {
                UUID id = UUIDTools.generateUUID(this.username);
                TokenSecurity.addClaim(this.claims, TokenDto.SECURITY_CLAIM_USER_ID, id.toString());
            }
        }

        if (riskRoleSet == false) {
            this.withRiskRole(RiskRole.NONE);
        }
        if (userRoleSet == false) {
            this.withUserRole(UserRole.ANYTHING);
        }
    }

    public TokenDto build() {
        if (this.username == null) {
            throw new RuntimeException("You must supply a username for token.");
        }

        reconcileClaims();

        String domain = StringTools.getDomainOrNull(username);
        if (this.reference == null) this.reference = domain;
        if (this.nameQualifier == null) this.nameQualifier = domain;

        TokenDto ret = TokenSecurity.generateToken(
                this.company,
                this.reference,
                this.username,
                this.nameQualifier,
                this.claims,
                this.expiresMins);

        if (shouldPublish) {
            AteDelegate.get().currentToken.publishToken(ret);
        }

        return ret;
    }
}
