package com.tokera.ate.events;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.io.api.IPartitionKey;

/**
 * Event thats triggered whenever the Token scope is entered
 */
public class TokenScopeChangedEvent {

    private IPartitionKey partitionKey;

    public TokenScopeChangedEvent(TokenDto token) {
        this.partitionKey = AteDelegate.get().headIO.tokenParser().extractPartitionKey(token);
    }

    public TokenScopeChangedEvent(IPartitionKey partitionKey) {
        this.partitionKey = partitionKey;
    }

    public IPartitionKey getPartitionKey() {
        return this.partitionKey;
    }

    public void setPartitionKey(IPartitionKey val) {
        this.partitionKey = val;
    }
}
