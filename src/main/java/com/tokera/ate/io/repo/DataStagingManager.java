package com.tokera.ate.io.repo;

import com.google.common.collect.Multimap;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.msg.MessagePublicKey;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageDataHeaderDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.io.api.IAteIO;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.core.PartitionKeyComparator;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.jboss.resteasy.specimpl.MultivaluedTreeMap;

import javax.enterprise.context.RequestScoped;
import javax.ws.rs.core.MultivaluedMap;
import java.util.*;
import java.util.stream.Collectors;

/**
 * Represents a staging area for storing data objects that are about to be saved or removed from data stores
 */
@RequestScoped
public class DataStagingManager {
    AteDelegate d = AteDelegate.get();

    private final Map<IPartitionKey, PartitionContext> partitionMergeContexts = new TreeMap<>(new PartitionKeyComparator());
    private final Set<IPartitionKey> touchedPartitions = new HashSet<>();

    public DataStagingManager() {
    }

    protected class PartitionContext {
        public List<UUID> toPutOrder = new ArrayList<>();
        public List<UUID> toDeleteOrder = new ArrayList<>();
        public Map<UUID, BaseDao> toPut = new HashMap<>();
        public Map<UUID, BaseDao> toDelete = new HashMap<>();
        public Map<String, MessagePublicKeyDto> savedPublicKeys = new HashMap<>();
        public Map<UUID, MessageDataDto> savedDatas = new HashMap<>();
    }

    private @Nullable PartitionContext getPartitionMergeContext(IPartitionKey key, boolean create)
    {
        PartitionContext context;
        if (this.partitionMergeContexts.containsKey(key) == false) {
            if (create == false) return null;
            context = new PartitionContext();
            this.partitionMergeContexts.put(key, context);
            return context;
        }

        context = this.partitionMergeContexts.get(key);
        assert context != null : "@AssumeAssertion(nullness): The section before ensures that the requestContext can never be null";
        return context;
    }

    public void clear() {
        partitionMergeContexts.clear();
    }

    public void put(IPartitionKey partitionKey, MessagePublicKeyDto key) {
        PartitionContext context = getPartitionMergeContext(partitionKey, true);
        if (key instanceof MessagePrivateKeyDto) {
            key = new MessagePublicKeyDto(key);
        }
        context.savedPublicKeys.put(key.getPublicKeyHash(), key);
    }

    public void put(PUUID key, MessageDataDto data) {
        PartitionContext context = getPartitionMergeContext(key.partition(), true);
        context.savedDatas.put(key.id(), data);
    }

    public void put(IPartitionKey partitionKey, UUID id, MessageDataDto data) {
        PartitionContext context = getPartitionMergeContext(partitionKey, true);
        context.savedDatas.put(id, data);
    }

    public void put(IPartitionKey partitionKey, BaseDao obj) {
        UUID id = obj.getId();
        PartitionContext context = getPartitionMergeContext(partitionKey, true);
        if (context.toPut.containsKey(id) == false) {
            context.toPut.put(id, obj);
            context.toPutOrder.add(id);
        }
        if (context.toDelete.remove(id) != null) {
            context.toDeleteOrder.remove(id);
        }
        if (touchedPartitions.contains(partitionKey) == false) {
            touchedPartitions.add(partitionKey);
        }
    }

    public void delete(IPartitionKey partitionKey, BaseDao obj) {
        UUID id = obj.getId();
        PartitionContext context = getPartitionMergeContext(partitionKey, true);
        if (context.toDelete.containsKey(id) == false) {
            context.toDelete.put(id, obj);
            context.toDeleteOrder.add(id);
        }
        if (context.toPut.remove(id) != null) {
            context.toPutOrder.remove(id);
        }
        if (touchedPartitions.contains(partitionKey) == false) {
            touchedPartitions.add(partitionKey);
        }
    }

    public void undo(IPartitionKey partitionKey, BaseDao obj) {
        UUID id = obj.getId();
        PartitionContext context = getPartitionMergeContext(partitionKey, true);
        if (context.toDelete.remove(id) != null) {
            context.toDeleteOrder.remove(id);
        }
        if (context.toPut.remove(id) != null) {
            context.toPutOrder.remove(id);
        }
    }

    public Iterable<IPartitionKey> keys() {
        return this.partitionMergeContexts.keySet().stream().collect(Collectors.toList());
    }

    public int size() {
        int ret = 0;
        for (PartitionContext context : this.partitionMergeContexts.values()) {
            ret += context.toPutOrder.size();
        }
        return ret;
    }

    public Iterable<BaseDao> puts(IPartitionKey partitionKey) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        return context.toPutOrder.stream().map(id -> context.toPut.get(id)).collect(Collectors.toList());
    }

    public Iterable<BaseDao> deletes(IPartitionKey partitionKey) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        return context.toDeleteOrder.stream().map(id -> context.toDelete.get(id)).collect(Collectors.toList());
    }

    public @Nullable BaseDao find(PUUID id) {
        return this.find(id.partition(), id.id());
    }

    public @Nullable BaseDao find(IPartitionKey partitionKey, UUID id) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return null;
        return context.toPut.getOrDefault(id, null);
    }

    public @Nullable MessagePublicKeyDto findPublicKey(IPartitionKey partitionKey, String publicKeyHash) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return null;
        MessagePublicKeyDto ret = MapTools.getOrNull(context.savedPublicKeys, publicKeyHash);
        if (ret == null) {
            ret = d.currentRights.findKey(publicKeyHash);
            if (ret == null) {
                ret = d.implicitSecurity.findEmbeddedKeyOrNull(publicKeyHash);
            }
        }
        return ret;
    }

    public @Nullable MessageDataDto findData(IPartitionKey partitionKey, UUID id) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return null;
        return MapTools.getOrNull(context.savedDatas, id);
    }

    public Set<IPartitionKey> getTouchedPartitions() {
        return this.touchedPartitions;
    }

    public Map<UUID, MessageDataDto> getSavedDataMap(IPartitionKey partitionKey) {
        return getPartitionMergeContext(partitionKey, true).savedDatas;
    }
}
