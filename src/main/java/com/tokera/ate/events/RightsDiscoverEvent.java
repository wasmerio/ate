package com.tokera.ate.events;

import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.HashSet;
import java.util.Set;

/**
 * This event is fired when the authorization system needs to discover what rights the current context (often the
 * current user) has within their possession. Other methods can be used to represent return the rights such as
 * private keys embedded in the token itself. Those that want to participate only need observe this event and then
 * update the properties held within
 */
public class RightsDiscoverEvent
{
    private @Nullable PrivateKeyWithSeedDto currentUserTrustWrite = null;
    private @Nullable PrivateKeyWithSeedDto currentUserTrustRead = null;
    private Set<PrivateKeyWithSeedDto> rightsRead = new HashSet<>();
    private Set<PrivateKeyWithSeedDto> rightsWrite = new HashSet<>();

    public @Nullable PrivateKeyWithSeedDto getCurrentUserTrustWrite() {
        return currentUserTrustWrite;
    }

    public void setCurrentUserTrustWrite(@Nullable PrivateKeyWithSeedDto currentUserTrustWrite) {
        this.currentUserTrustWrite = currentUserTrustWrite;
    }

    public @Nullable PrivateKeyWithSeedDto getCurrentUserTrustRead() {
        return currentUserTrustRead;
    }

    public void setCurrentUserTrustRead(@Nullable PrivateKeyWithSeedDto currentUserTrustRead) {
        this.currentUserTrustRead = currentUserTrustRead;
    }

    public Set<PrivateKeyWithSeedDto> getRightsRead() {
        return rightsRead;
    }

    public void setRightsRead(Set<PrivateKeyWithSeedDto> rightsRead) {
        this.rightsRead = rightsRead;
    }

    public Set<PrivateKeyWithSeedDto> getRightsWrite() {
        return rightsWrite;
    }

    public void setRightsWrite(Set<PrivateKeyWithSeedDto> rightsWrite) {
        this.rightsWrite = rightsWrite;
    }
}































