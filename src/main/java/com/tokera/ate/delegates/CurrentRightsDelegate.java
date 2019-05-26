package com.tokera.ate.delegates;

import com.tokera.ate.dao.IRights;
import com.tokera.ate.dao.IRoles;
import com.tokera.ate.events.NewAccessRightsEvent;
import com.tokera.ate.events.RightsDiscoverEvent;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.units.*;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;

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
    private @Nullable Set<MessagePrivateKeyDto> rightsReadCache = null;
    private @Nullable Set<MessagePrivateKeyDto> rightsWriteCache = null;
    private @Nullable Set<MessagePrivateKeyDto> impersonateRead = null;
    private @Nullable Set<MessagePrivateKeyDto> impersonateWrite = null;
    
    public CurrentRightsDelegate() {
    }
    
    public void init(@Observes NewAccessRightsEvent event)
    {
        // Remove any existing permissions we gains (if we still have the right
        // to them then we will get another copy)
        rightsReadCache = null;
        rightsWriteCache = null;
        impersonateRead = null;
        impersonateWrite = null;

        // Fire an event that will discover all the authorization rights
        currentRights = new RightsDiscoverEvent();
        d.eventRightsDiscover.fire(currentRights);

        clearRightsCache();
    }
    
    public void clearRightsCache() {
        rightsReadCache = null;
        rightsWriteCache = null;
    }
    
    public void impersonate(IPartitionKey key, IRights rights) {
        d.requestContext.pushPartitionKey(key);
        try {
            d.authorization.getOrCreateImplicitRightToRead(rights);
            d.authorization.getOrCreateImplicitRightToWrite(rights);

            this.impersonateRead = rights.getRightsRead();
            this.impersonateWrite = rights.getRightsWrite();
        } finally {
            d.requestContext.popPartitionKey();
        }
        
        clearRightsCache();
    }

    public void impersonate(IRights rights) {
        IPartitionKey key = d.headIO.partitionResolver().resolve(rights);
        impersonate(key, rights);
    }

    @Override
    public @DaoId UUID getId() {
        TokenDto token = d.currentToken.getTokenOrNull();
        if (token == null) throw new WebApplicationException("There is no currentRights user in the requestContext.");
        return token.getUserId();
    }

    @Override
    public Set<MessagePrivateKeyDto> getRightsRead() {
        if (this.rightsReadCache != null) {
            return this.rightsReadCache;
        }
        
        boolean shouldCache = true;
        Set<MessagePrivateKeyDto> ret = new HashSet<>();
        
        if (d.currentToken.getWithinTokenScope() == true) {
            ret.addAll(this.d.tokenSecurity.getRightsRead());
        }
        
        ret.addAll(currentRights.getRolesRead());

        MessagePrivateKeyDto currentUserRead = currentRights.getCurrentUserTrustRead();
        if (currentUserRead != null) {
            ret.add(currentUserRead);
        } else {
            shouldCache = false;
        }
        
        if (impersonateRead != null) {
            for (MessagePrivateKeyDto key : impersonateRead) {
                if (ret.contains(key) == false) {
                    ret.add(key);
                }
            }
        } else {
            shouldCache = false;
        }
        
        if (ret.contains(new MessagePrivateKeyDto(d.encryptor.getTrustOfPublicRead())) == false) {
            ret.add(new MessagePrivateKeyDto(d.encryptor.getTrustOfPublicRead()));
        }
        
        if (shouldCache == true) {
            this.rightsReadCache = ret.stream().collect(Collectors.toSet());
        }
        return ret;
    }

    @Override
    public Set<MessagePrivateKeyDto> getRightsWrite() {
        if (this.rightsWriteCache != null) {
            return this.rightsWriteCache;
        }
        
        boolean shouldCache = true;
        Set<MessagePrivateKeyDto> ret = new HashSet<>();
        
        if (d.currentToken.getWithinTokenScope() == true) {
            ret.addAll(this.d.tokenSecurity.getRightsWrite());
        }
        
        ret.addAll(this.currentRights.getRolesWrite());

        MessagePrivateKeyDto currentUserWrite = currentRights.getCurrentUserTrustWrite();
        if (currentUserWrite != null) {
            ret.add(currentUserWrite);
        } else {
            shouldCache = false;
        }
        
        if (impersonateWrite != null) {
            for (MessagePrivateKeyDto key : impersonateWrite) {
                if (ret.contains(key) == false) {
                    ret.add(key);
                }
            }
        } else {
            shouldCache = false;
        }
        
        if (ret.contains(new MessagePrivateKeyDto(d.encryptor.getTrustOfPublicWrite())) == false) {
            ret.add(new MessagePrivateKeyDto(d.encryptor.getTrustOfPublicWrite()));
        }
                
        if (shouldCache == true) {
            this.rightsWriteCache = ret.stream().collect(Collectors.toSet());
        }
        return ret;
    }
    
    @Override
    public @Alias String getRightsAlias() {
        TokenDto token = d.currentToken.getTokenOrNull();
        if (token == null) {
            throw new UnsupportedOperationException("No token attached to this session.");
        }
        return (@Alias String)token.getUsername();
    }

    @Override
    public void onAddRight(IRoles to) {
    }

    @Override
    public void onRemoveRight(IRoles from) {
    }
}