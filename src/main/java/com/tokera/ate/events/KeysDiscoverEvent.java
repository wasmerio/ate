package com.tokera.ate.events;

import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.io.api.IPartitionKey;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

/**
 * This event is fired when a new topic is created and it must be seeded with public keys that are intrinsic to the
 * specific use-case of ATE
 */
public class KeysDiscoverEvent
{
    private IPartitionKey partitionKey;
    private List<MessagePublicKeyDto> keys;

    public KeysDiscoverEvent(IPartitionKey key) {
        this.partitionKey = key;
        this.keys = new ArrayList<>();
    }

    public IPartitionKey getPartitionKey() {
        return partitionKey;
    }

    public void setPartitionKey(IPartitionKey partitionKey) {
        this.partitionKey = partitionKey;
    }

    public List<MessagePublicKeyDto> getKeys() {
        return keys;
    }

    public void setKeys(List<MessagePublicKeyDto> keys) {
        this.keys = keys;
    }
}