package com.tokera.ate.delegates;

import com.tokera.ate.dao.IRights;
import com.tokera.ate.dao.IRoles;
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.events.NewAccessRightsEvent;
import com.tokera.ate.events.RightsDiscoverEvent;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.units.*;
import com.tokera.ate.dto.TokenDto;

import java.util.*;
import java.util.stream.Collectors;
import javax.enterprise.context.RequestScoped;
import javax.enterprise.event.Observes;
import javax.ws.rs.WebApplicationException;

import org.checkerframework.checker.nullness.qual.Nullable;

/**
 * Delegate used to interact with the key metadata around the currentRights made to the API
 */
@RequestScoped
public class CurrentRightsDelegate implements IRights {

    private AteDelegate d = AteDelegate.get();
    private RightsDiscoverEvent currentRights = new RightsDiscoverEvent();
    private @Nullable Set<PrivateKeyWithSeedDto> rightsReadCache = null;
    private @Nullable Set<PrivateKeyWithSeedDto> rightsWriteCache = null;
    private final Set<PrivateKeyWithSeedDto> impersonateRead = new HashSet<>();
    private final Set<PrivateKeyWithSeedDto> impersonateWrite = new HashSet<>();
    private String readRightsHash = null;
    
    public CurrentRightsDelegate() {
    }
    
    public void init(@Observes NewAccessRightsEvent event)
    {
        // Remove any existing permissions we gains (if we still have the right
        // to them then we will getData another copy)
        rightsReadCache = null;
        rightsWriteCache = null;

        // Fire an event that will discover all the authorization rights
        currentRights = new RightsDiscoverEvent();
        d.eventRightsDiscover.fire(currentRights);

        clearRightsCache();
    }
    
    public void clearRightsCache() {
        this.rightsReadCache = null;
        this.rightsWriteCache = null;
        this.readRightsHash = null;
    }

    public void clearImpersonation() {
        this.impersonateRead.clear();
        this.impersonateWrite.clear();
        clearRightsCache();
    }
    
    public void impersonate(IPartitionKey key, IRights rights) {
        d.requestContext.pushPartitionKey(key);
        try {
            d.authorization.getOrCreateImplicitRightToRead(rights);
            d.authorization.getOrCreateImplicitRightToWrite(rights);

            for (PrivateKeyWithSeedDto right : rights.getRightsRead()) {
                this.impersonateRead.add(right);
            }
            for (PrivateKeyWithSeedDto right : rights.getRightsWrite()) {
                this.impersonateWrite.add(right);
            }
        } finally {
            d.requestContext.popPartitionKey();
        }
        clearRightsCache();
    }

    public void impersonateRead(PrivateKeyWithSeedDto key) {
        this.impersonateRead.add(key);
        clearRightsCache();
    }

    public void impersonateWrite(PrivateKeyWithSeedDto key) {
        this.impersonateWrite.add(key);
        clearRightsCache();
    }

    public boolean unimpersonateRead(PrivateKeyWithSeedDto key) {
        boolean ret = this.impersonateRead.remove(key);
        clearRightsCache();
        return ret;
    }

    public boolean unimpersonateWrite(PrivateKeyWithSeedDto key) {
        boolean ret = this.impersonateWrite.remove(key);
        clearRightsCache();
        return ret;
    }

    public void impersonate(IRights rights) {
        IPartitionKey key = d.io.partitionResolver().resolveOrThrow(rights);
        impersonate(key, rights);
    }

    @Override
    public @DaoId UUID getId() {
        TokenDto token = d.currentToken.getTokenOrNull();
        if (token == null) throw new WebApplicationException("There is no current user in the request.");
        return token.getUserId();
    }

    @SuppressWarnings("known.nonnull")
    @Override
    public Set<PrivateKeyWithSeedDto> getRightsRead() {
        if (this.rightsReadCache != null) {
            return this.rightsReadCache;
        }

        boolean shouldCache = true;
        Set<PrivateKeyWithSeedDto> ret = new HashSet<>();

        if (d.currentToken.getWithinTokenScope() == true) {
            ret.addAll(this.d.tokenSecurity.getRightsRead());
        }

        ret.addAll(currentRights.getRightsRead());

        PrivateKeyWithSeedDto currentUserRead = currentRights.getCurrentUserTrustRead();
        if (currentUserRead != null) {
            ret.add(currentUserRead);
        } else {
            shouldCache = false;
        }

        if (impersonateRead != null) {
            ret.addAll(this.impersonateRead);
        } else {
            shouldCache = false;
        }

        PrivateKeyWithSeedDto publicRead = new PrivateKeyWithSeedDto(d.encryptor.getTrustOfPublicRead());
        ret.add(publicRead);

        if (shouldCache == true) {
            this.rightsReadCache = ret;
        }
        return ret;
    }

    @SuppressWarnings("known.nonnull")
    @Override
    public Set<PrivateKeyWithSeedDto> getRightsWrite() {
        if (this.rightsWriteCache != null) {
            return this.rightsWriteCache;
        }

        boolean shouldCache = true;
        Set<PrivateKeyWithSeedDto> ret = new HashSet<>();

        if (d.currentToken.getWithinTokenScope() == true) {
            ret.addAll(this.d.tokenSecurity.getRightsWrite());
        }

        ret.addAll(this.currentRights.getRightsWrite());

        PrivateKeyWithSeedDto currentUserWrite = currentRights.getCurrentUserTrustWrite();
        if (currentUserWrite != null) {
            ret.add(currentUserWrite);
        } else {
            shouldCache = false;
        }

        if (impersonateWrite != null) {
            ret.addAll(impersonateWrite);
        } else {
            shouldCache = false;
        }

        PrivateKeyWithSeedDto publicWrite = new PrivateKeyWithSeedDto(d.encryptor.getTrustOfPublicWrite());
        ret.add(publicWrite);

        if (shouldCache == true) {
            this.rightsWriteCache = ret;
        }
        return ret;
    }

    public String computeReadRightsHash() {
        if (readRightsHash != null) {
            return readRightsHash;
        }

        readRightsHash = d.encryptor.hashMd5AndEncode(this.getRightsRead().stream()
                .map(r -> r.seed().getBytes())
                .collect(Collectors.toList()));
        return readRightsHash;
    }

    public @Nullable MessagePublicKeyDto findKeyAndConvertToPublic(String publicKeyHash) {
        PrivateKeyWithSeedDto ret = findKey(publicKeyHash);
        if (ret == null) return null;
        return new MessagePublicKeyDto(ret);
    }

    public @Nullable PrivateKeyWithSeedDto findKey(String publicKeyHash) {
        for (PrivateKeyWithSeedDto key : this.getRightsRead()) {
            if (publicKeyHash.equals(key.publicHash())) {
                return key;
            }
        }
        for (PrivateKeyWithSeedDto key : this.getRightsWrite()) {
            if (publicKeyHash.equals(key.publicHash())) {
                return key;
            }
        }
        return null;
    }
    
    @Override
    public @Alias String getRightsAlias() {
        TokenDto token = d.currentToken.getTokenOrNull();
        if (token == null) {
            throw new UnsupportedOperationException("No token attached to this session.");
        }
        return token.getUsername();
    }

    @Override
    public void onAddRight(IRoles to) {
    }

    @Override
    public void onRemoveRight(IRoles from) {
    }

    @Override
    public boolean readOnly() {
        return true;
    }

    public @Nullable PrivateKeyWithSeedDto findReadKey(String publicKeyHash)
    {
        return this.getRightsRead().stream()
                .filter(k -> publicKeyHash.equals(k.publicHash()))
                .findFirst()
                .orElse(null);
    }

    public @Nullable PrivateKeyWithSeedDto findWriteKey(String publicKeyHash)
    {
        return this.getRightsWrite().stream()
                .filter(k -> publicKeyHash.equals(k.publicHash()))
                .findFirst()
                .orElse(null);
    }
}