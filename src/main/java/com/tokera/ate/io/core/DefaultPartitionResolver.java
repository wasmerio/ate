package com.tokera.ate.io.core;

import com.google.common.cache.Cache;
import com.google.common.cache.CacheBuilder;
import com.tokera.ate.dao.IRights;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.base.BaseDaoInternal;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.api.IPartitionResolver;
import com.tokera.ate.units.DaoId;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import java.util.UUID;
import java.util.concurrent.TimeUnit;

/**
 * Default implementation of the partition resolver which will use a hashing algorithm on the primary
 * key of the root of the tree to determine the partition that data will be mapped to.
 */
public class DefaultPartitionResolver implements IPartitionResolver {
    private AteDelegate d = AteDelegate.get();

    private @Nullable IPartitionKey resolveInternal(BaseDao obj, boolean shouldThrow)
    {
        // If the object implements the partition key interface then it can define its own partition key
        if (obj instanceof IPartitionKey) {
            return (IPartitionKey) obj;
        }

        // Check the object itself
        IPartitionKey partitionKey = BaseDaoInternal.getPartitionKey(obj);
        if (partitionKey != null) return partitionKey;

        // Follow the chain-of-trust up to the root of the tree
        @DaoId UUID parentId = obj.getParentId();
        if (parentId == null)
        {
            Class<?> type = obj.getClass();
            if (d.daoParents.getAllowedParentFree().contains(type) == false) {
                if (type.getAnnotation(Dependent.class) == null) {
                    if (shouldThrow == false) return null;
                    throw new RuntimeException("This entity [" + type.getSimpleName() + "] has not been marked with the Dependent annotation.");
                }
                if (shouldThrow == false) return null;
                throw new RuntimeException("This entity [" + type.getSimpleName() + "] is not attached to a parent [see PermitParentType annotation].");
            }

            // We have arrived at the top of the chain-of-trust and thus the ID of this root object
            // can be used to determine which partition to place the data object
            return d.io.partitionKeyMapper().resolve(obj.getId());
        }

        // If it has a parent then we need to grab the partition key of the parent rather than this object
        // otherwise the chain of trust will get distributed to different partitions which would break the
        // design goals
        BaseDao next = d.memoryRequestCacheIO.getOrNull(obj.getParentId());
        if (next == null)
        {
            // Try all the partition keys that are currently active or that have not yet been saved
            for (IPartitionKey activePartitionKey : d.dataStagingManager.keys()) {
                if (d.io.exists(PUUID.from(activePartitionKey, parentId))) {
                    return activePartitionKey;
                }
                if (d.dataStagingManager.find(activePartitionKey, parentId) != null) {
                    return activePartitionKey;
                }
            }

            // We can't find it in the active data set but perhaps its a part of the current partition key
            // scope
            partitionKey = d.requestContext.getPartitionKeyScopeOrNull();
            if (partitionKey != null) {
                if (d.io.exists(PUUID.from(partitionKey, parentId))) {
                    return partitionKey;
                }
            }

            // Lets try some other partition scopes (perhaps its in one of those)
            for (IPartitionKey otherPartitionKey : d.requestContext.getOtherPartitionKeys()) {
                if (d.io.exists(PUUID.from(otherPartitionKey, parentId))) {
                    return otherPartitionKey;
                }
            }

            // This object isn't known to the current context so we really can't do much with it
            if (shouldThrow == false) return null;
            throw new RuntimeException("Unable to transverse up the tree high enough to determine the topic and partition for this data object [" + obj + "].");
        }

        // Return the partition key of the parent
        return next.partitionKey(shouldThrow);
    }

    @Override
    public IPartitionKey resolveOrThrow(BaseDao obj) {
        return resolveInternal(obj, true);
    }

    @Override
    public IPartitionKey resolveOrThrow(IRights obj) {
        if (obj instanceof BaseDao) {
            return ((BaseDao) obj).partitionKey(true);
        }
        throw new RuntimeException("Unable to determine the partition key for this access rights object as it is not of the type BaseDao.");
    }

    @Override
    public @Nullable IPartitionKey resolveOrNull(BaseDao obj) {
        return resolveInternal(obj, false);
    }

    @Override
    public @Nullable IPartitionKey resolveOrNull(IRights obj) {
        if (obj instanceof BaseDao) {
            return ((BaseDao)obj).partitionKey(false);
        }
        return null;
    }
}