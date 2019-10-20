package com.tokera.ate.delegates;

import com.tokera.ate.io.api.IPartitionKey;

import javax.enterprise.context.ApplicationScoped;
import java.util.UUID;

@ApplicationScoped
public class InvalidationDelegate {
    private final AteDelegate d = AteDelegate.get();

    public void invalidate(String clazzName, IPartitionKey partKey, UUID id) {
        d.permissionCache.invalidate(clazzName, partKey, id);
        d.indexing.invalidate(clazzName, partKey, id);
    }
}
