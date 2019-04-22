package com.tokera.ate.events;

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
    private @Nullable MessagePrivateKeyDto currentUserTrustWrite = null;
    private @Nullable MessagePrivateKeyDto currentUserTrustRead = null;
    private Set<MessagePrivateKeyDto> rolesRead = new HashSet<>();
    private Set<MessagePrivateKeyDto> rolesWrite = new HashSet<>();

    public @Nullable MessagePrivateKeyDto getCurrentUserTrustWrite() {
        return currentUserTrustWrite;
    }

    public void setCurrentUserTrustWrite(@Nullable MessagePrivateKeyDto currentUserTrustWrite) {
        this.currentUserTrustWrite = currentUserTrustWrite;
    }

    public @Nullable MessagePrivateKeyDto getCurrentUserTrustRead() {
        return currentUserTrustRead;
    }

    public void setCurrentUserTrustRead(@Nullable MessagePrivateKeyDto currentUserTrustRead) {
        this.currentUserTrustRead = currentUserTrustRead;
    }

    public Set<MessagePrivateKeyDto> getRolesRead() {
        return rolesRead;
    }

    public void setRolesRead(Set<MessagePrivateKeyDto> rolesRead) {
        this.rolesRead = rolesRead;
    }

    public Set<MessagePrivateKeyDto> getRolesWrite() {
        return rolesWrite;
    }

    public void setRolesWrite(Set<MessagePrivateKeyDto> rolesWrite) {
        this.rolesWrite = rolesWrite;
    }
}































