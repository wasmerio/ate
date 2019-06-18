package com.tokera.ate.events;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.io.api.IPartitionKey;
import org.checkerframework.checker.nullness.qual.Nullable;

/**
 * Event thats triggered whenever the Token scope is entered
 */
public class TokenScopeChangedEvent {

    private TokenDto token;
    private @Nullable IPartitionKey partitionKey;

    public TokenScopeChangedEvent(TokenDto token) {
        this.token = token;
        this.partitionKey = AteDelegate.get().io.tokenParser().extractPartitionKey(token);
    }

    public TokenScopeChangedEvent(IPartitionKey partitionKey) {
        this.partitionKey = partitionKey;
    }

    public @Nullable IPartitionKey getPartitionKey() {
        return this.partitionKey;
    }

    public void setPartitionKey(@Nullable IPartitionKey val) {
        this.partitionKey = val;
    }

    public TokenDto getToken() {
        return token;
    }

    public void setToken(TokenDto token) {
        this.token = token;
    }
}
