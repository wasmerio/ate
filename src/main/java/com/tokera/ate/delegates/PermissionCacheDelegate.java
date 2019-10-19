package com.tokera.ate.delegates;

import com.google.common.cache.Cache;
import com.google.common.cache.CacheBuilder;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.dao.enumerations.PermissionPhase;
import com.tokera.ate.dto.EffectivePermissions;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.core.PartitionKeyComparator;
import com.tokera.ate.scopes.Startup;
import com.tokera.ate.security.EffectivePermissionBuilder;
import com.tokera.ate.units.DaoId;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import java.util.UUID;
import java.util.concurrent.ExecutionException;

/**
 * Delegate used to cache permissions
 */
@Startup
@ApplicationScoped
public class PermissionCacheDelegate {
    private final AteDelegate d = AteDelegate.get();
    private static PartitionKeyComparator partitionKeyComparator = new PartitionKeyComparator();

    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;

    private class CacheKey implements Comparable<CacheKey> {
        public final UUID id;
        public final IPartitionKey partKey;
        public final String clazz;

        public CacheKey(IPartitionKey partKey, @Nullable String clazz, UUID id) {
            this.partKey = partKey;
            this.clazz = clazz;
            this.id = id;
        }

        @Override
        public int compareTo(CacheKey other) {
            int diff = id.compareTo(other.id);
            if (diff != 0) return diff;
            diff = partitionKeyComparator.compare(partKey, other.partKey);
            if (diff != 0) return diff;
            diff = clazz.compareTo(other.clazz);
            if (diff != 0) return diff;
            return 0;
        }

        @Override
        public int hashCode() {
            int hashCode = this.partKey.hashCode();
            hashCode = 37 * hashCode + this.clazz.hashCode();
            hashCode = 37 * hashCode + this.id.hashCode();
            return hashCode;
        }

        @Override
        public boolean equals(Object other) {
            if (other instanceof CacheKey) {
                return compareTo((CacheKey)other) == 0;
            }
            return false;
        }
    }

    private Cache<CacheKey, EffectivePermissions> permissionsCache = CacheBuilder.newBuilder()
            .maximumSize(d.bootstrapConfig.getPermissionsCacheLimit())
            .build();

    public void invalidate(String type, IPartitionKey partitionKey, @DaoId UUID id) {
        this.permissionsCache.invalidate(new CacheKey(partitionKey, type, id));
    }

    private EffectivePermissions computePerms(String type, IPartitionKey partitionKey, @DaoId UUID id, PermissionPhase phase)
    {
        return new EffectivePermissionBuilder(type, partitionKey, id)
                .withPhase(phase)
                .build();
    }

    public EffectivePermissions perms(String type, IPartitionKey partitionKey, @DaoId UUID id, PermissionPhase phase)
    {
        if (type == null) return computePerms(type, partitionKey, id, phase);
        if (phase != PermissionPhase.BeforeMerge) return computePerms(type, partitionKey, id, phase);

        CacheKey cacheKey = new CacheKey(partitionKey, type, id);
        try {
            return permissionsCache.get(cacheKey, () -> computePerms(type, partitionKey, id, phase));
        } catch (ExecutionException e) {
            throw new RuntimeException(e);
        }
    }
}
