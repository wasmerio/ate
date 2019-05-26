package com.tokera.ate.io.repo;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.io.api.IAteIO;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.core.PartitionKeyComparator;

import javax.enterprise.context.RequestScoped;
import java.util.*;

/**
 * Represents a staging area for storing data objects that are about to be saved or removed from data stores
 */
@RequestScoped
public class DataStagingManager {
    private final Map<IPartitionKey, PartitionContext> partitionMergeContexts = new TreeMap<>(new PartitionKeyComparator());

    public DataStagingManager() {

    }

    public class PartitionContext {
        public HashSet<UUID> toPutKeys = new HashSet<>();
        public List<BaseDao> toPut = new ArrayList<>();
        public HashSet<UUID> toDeleteKeys = new HashSet<>();
        public List<BaseDao> toDelete = new ArrayList<>();
    }

    public PartitionContext getPartitionMergeContext(IPartitionKey key)
    {
        PartitionContext context;
        if (this.partitionMergeContexts.containsKey(key) == false) {
            context = new PartitionContext();
            this.partitionMergeContexts.put(key, context);
            return context;
        }

        context = this.partitionMergeContexts.get(key);
        assert context != null : "@AssumeAssertion(nullness): The section before ensures that the requestContext can never be null";
        return context;
    }

    public Set<IPartitionKey> getActivePartitionKeys() {
        return partitionMergeContexts.keySet();
    }

    public void clear() {
        partitionMergeContexts.clear();
    }
}
