package com.tokera.ate.io.repo;

import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.base.BaseDaoInternal;
import com.tokera.ate.dao.kafka.MessageSerializer;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.core.PartitionKeyComparator;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.*;
import java.util.stream.Collectors;

/**
 * Represents a staging area for storing data objects that are about to be saved or removed from data stores
 */
public class DataTransaction {
    AteDelegate d = AteDelegate.get();

    private final boolean shouldSync;
    private final Map<IPartitionKey, PartitionContext> partitions = new TreeMap<>(new PartitionKeyComparator());
    private final Map<IPartitionKey, PartitionCache> cache = new TreeMap<>(new PartitionKeyComparator());

    @SuppressWarnings("initialization.fields.uninitialized")
    private DataSubscriber subscriber;

    public DataTransaction(boolean sync) {
        this.shouldSync = sync;
        this.subscriber = d.storageFactory.get().backend();
    }

    public void copyCacheFrom(DataTransaction other) {
        for (IPartitionKey key : other.partitions.keySet()) {
            PartitionContext otherContext = other.partitions.get(key);
            PartitionContext myContext = getPartitionMergeContext(key, true);

            myContext.savedWriteKeys.putAll(otherContext.savedWriteKeys);
            myContext.savedDatas.putAll(otherContext.savedDatas);
            myContext.savedPublicKeys.putAll(otherContext.savedPublicKeys);

            for (UUID id : otherContext.savedDatas.keySet()) {
                myContext.toPut.remove(id);
                myContext.toPutOrder.remove(id);
                myContext.toDelete.remove(id);
                myContext.toDeleteOrder.remove(id);
            }
        }

        for (IPartitionKey key : other.cache.keySet()) {
            PartitionCache otherCache = other.cache.get(key);
            PartitionCache myCache = this.getPartitionCache(key);
            myCache.entries.putAll(otherCache.entries);
            myCache.publicKeys.putAll(otherCache.publicKeys);
            myCache.castles.putAll(otherCache.castles);
        }
    }

    /**
     * Context of the transaction to a particular partition
     */
    protected class PartitionContext {
        public final List<UUID> toPutOrder = new ArrayList<>();
        public final List<UUID> toDeleteOrder = new ArrayList<>();
        public final Map<UUID, BaseDao> toPut = new HashMap<>();
        public final Map<UUID, BaseDao> toDelete = new HashMap<>();
        public final Map<String, MessagePrivateKeyDto> savedWriteKeys = new HashMap<>();
        public final Map<String, MessagePublicKeyDto> savedPublicKeys = new HashMap<>();
        public final Map<UUID, MessageDataDto> savedDatas = new HashMap<>();
    }

    protected class PartitionCache {
        public final Map<UUID, BaseDao> entries = new HashMap<>();
        public final Map<String, MessagePublicKeyDto> publicKeys = new HashMap<>();
        public final Map<String, MessageSecurityCastleDto> castles = new HashMap<>();
    }

    private @Nullable PartitionContext getPartitionMergeContext(IPartitionKey key, boolean create)
    {
        PartitionContext context;
        if (this.partitions.containsKey(key) == false) {
            if (create == false) return null;
            context = new PartitionContext();
            this.partitions.put(key, context);
            return context;
        }

        context = this.partitions.get(key);
        assert context != null : "@AssumeAssertion(nullness): The section before ensures that the requestContext can never be null";
        return context;
    }

    private PartitionCache getPartitionCache(IPartitionKey partitionKey) {
        if (this.cache.containsKey(partitionKey) == true) {
            return this.cache.get(partitionKey);
        }

        PartitionCache ret = new PartitionCache();
        this.cache.put(partitionKey, ret);
        return ret;
    }

    void put(IPartitionKey partitionKey, MessagePublicKeyDto key) {
        if (key instanceof MessagePrivateKeyDto) {
            key = new MessagePublicKeyDto(key);
        }
        cache(partitionKey, key);
    }

    void put(IPartitionKey partitionKey, Set<MessagePrivateKeyDto> keys) {
        PartitionContext context = getPartitionMergeContext(partitionKey, true);
        Map<String, MessagePrivateKeyDto> keysMap = context.savedWriteKeys;
        keys.forEach(k -> {
            if (keysMap.containsKey(k.getPublicKeyHash()) == false) {
                keysMap.put(k.getPublicKeyHash(), k);
            }
            cache(partitionKey, new MessagePublicKeyDto(k));
        });
    }

    void wrote(IPartitionKey partitionKey, MessageBaseDto msg) {
        PartitionContext context = getPartitionMergeContext(partitionKey, true);
        if (msg instanceof MessageDataDto) {
            MessageDataDto data = (MessageDataDto)msg;
            context.savedDatas.put(data.getHeader().getIdOrThrow(), data);
        }
        if (msg instanceof MessagePublicKeyDto) {
            MessagePublicKeyDto key = (MessagePublicKeyDto)msg;
            context.savedPublicKeys.put(key.getPublicKeyHash(), key);
        }
    }

    void put(IPartitionKey partitionKey, BaseDao obj) {
        UUID id = obj.getId();
        PartitionContext context = getPartitionMergeContext(partitionKey, true);
        if (context.toPut.containsKey(id) == false) {
            context.toPut.put(id, obj);
            context.toPutOrder.add(id);
        }
        if (context.toDelete.remove(id) != null) {
            context.toDeleteOrder.remove(id);
        }

        cache(partitionKey, obj);
    }

    void delete(IPartitionKey partitionKey, BaseDao obj) {
        UUID id = obj.getId();
        PartitionContext context = getPartitionMergeContext(partitionKey, true);
        if (context.toDelete.containsKey(id) == false) {
            context.toDelete.put(id, obj);
            context.toDeleteOrder.add(id);
        }
        if (context.toPut.remove(id) != null) {
            context.toPutOrder.remove(id);
        }

        uncache(partitionKey, obj.getId());
    }

    void undo(IPartitionKey partitionKey, BaseDao obj) {
        UUID id = obj.getId();
        PartitionContext context = getPartitionMergeContext(partitionKey, true);
        if (context.toDelete.remove(id) != null) {
            context.toDeleteOrder.remove(id);
        }
        if (context.toPut.remove(id) != null) {
            context.toPutOrder.remove(id);
        }

        uncache(partitionKey, obj.getId());
    }

    public void cache(IPartitionKey partitionKey, BaseDao entity)
    {
        PartitionCache c = this.getPartitionCache(partitionKey);
        c.entries.put(entity.getId(), entity);
    }

    public void cache(IPartitionKey partitionKey, MessagePublicKeyDto t) {
        PartitionCache c = this.getPartitionCache(partitionKey);
        c.publicKeys.put(MessageSerializer.getKey(t), t);
    }

    public void cache(IPartitionKey partitionKey, MessageSecurityCastleDto t) {
        PartitionCache c = this.getPartitionCache(partitionKey);
        c.castles.put(MessageSerializer.getKey(t), t);
    }

    public boolean uncache(PUUID id)
    {
        return uncache(id.partition(), id.id());
    }

    public boolean uncache(IPartitionKey partitionKey, UUID id)
    {
        PartitionCache c = this.getPartitionCache(partitionKey);
        return c.entries.remove(id) != null;
    }

    public Collection<IPartitionKey> keys() {
        return this.partitions.keySet().stream().collect(Collectors.toList());
    }

    public int size() {
        int ret = 0;
        for (PartitionContext context : this.partitions.values()) {
            ret += context.toPutOrder.size();
        }
        return ret;
    }

    Iterable<BaseDao> puts(IPartitionKey partitionKey) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        return context.toPutOrder.stream().map(id -> context.toPut.get(id)).collect(Collectors.toList());
    }

    Iterable<BaseDao> deletes(IPartitionKey partitionKey) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        return context.toDeleteOrder.stream().map(id -> context.toDelete.get(id)).collect(Collectors.toList());
    }

    public boolean written(PUUID id) {
        return this.exists(id.partition(), id.id());
    }

    public boolean written(IPartitionKey partitionKey, UUID id)
    {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return false;
        return context.toPut.containsKey(id);
    }

    public boolean exists(PUUID id) {
        return this.exists(id.partition(), id.id());
    }

    public boolean exists(IPartitionKey partitionKey, UUID id)
    {
        if (this.cache.containsKey(partitionKey)) {
            PartitionCache cache = this.cache.get(partitionKey);
            if (cache.entries.containsKey(id)) return true;
        }

        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return false;
        return context.toPut.containsKey(id);
    }

    public @Nullable BaseDao find(PUUID id) {
        return this.find(id.partition(), id.id());
    }

    public @Nullable BaseDao find(IPartitionKey partitionKey, UUID id)
    {
        if (this.cache.containsKey(partitionKey)) {
            PartitionCache cache = this.cache.get(partitionKey);
            BaseDao ret = MapTools.getOrNull(cache.entries, id);
            if (ret != null) return ret;
        }

        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return null;
        return context.toPut.getOrDefault(id, null);
    }

    public @Nullable MessagePublicKeyDto findPublicKey(IPartitionKey partitionKey, String publicKeyHash) {
        if (this.cache.containsKey(partitionKey)) {
            PartitionCache cache = this.cache.get(partitionKey);
            MessagePublicKeyDto ret = MapTools.getOrNull(cache.publicKeys, publicKeyHash);
            if (ret != null) return ret;
        }

        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return null;
        MessagePublicKeyDto ret = d.currentRights.findKey(publicKeyHash);
        if (ret == null) {
            ret = d.implicitSecurity.findEmbeddedKeyOrNull(publicKeyHash);
        }
        return ret;
    }

    Collection<MessagePublicKeyDto> findPublicKeys(IPartitionKey partitionKey) {
        if (this.cache.containsKey(partitionKey)) {
            PartitionCache cache = this.cache.get(partitionKey);
            return cache.publicKeys.values();
        }

        return new LinkedList<>();
    }

    public @Nullable MessagePrivateKeyDto findPrivateKey(IPartitionKey partitionKey, String publicKeyHash) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return null;
        return MapTools.getOrNull(context.savedWriteKeys, publicKeyHash);
    }

    Collection<MessagePrivateKeyDto> findPrivateKeys(IPartitionKey partitionKey) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return new LinkedList<>();
        return context.savedWriteKeys.values();
    }

    public @Nullable MessageDataDto findSavedData(IPartitionKey partitionKey, UUID id) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return null;
        return MapTools.getOrNull(context.savedDatas, id);
    }

    public @Nullable MessagePublicKeyDto findSavedPublicKey(IPartitionKey partitionKey, String hash) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return null;
        return MapTools.getOrNull(context.savedPublicKeys, hash);
    }

    Map<UUID, MessageDataDto> getSavedDataMap(IPartitionKey partitionKey) {
        return getPartitionMergeContext(partitionKey, true).savedDatas;
    }

    /**
     * Deletes an object when the transaction is flushed
     */
    public void delete(BaseDao entity) {
        IPartitionKey partitionKey = entity.partitionKey(true);

        // We only actually need to validate and queue if the object has ever been saved
        if (BaseDaoInternal.hasSaved(entity) == true) {
            d.dataRepository.validateTrustStructure(entity);
            d.dataRepository.validateTrustPublicKeys(this, entity);

            d.requestContext.currentTransaction().put(partitionKey, d.currentRights.getRightsWrite());
            delete(partitionKey, entity);
        } else {
            undo(partitionKey, entity);
        }

        uncache(partitionKey, entity.getId());
    }

    /**
     * Writes a data object to this transaction which will be commited to the database along with the whole transaction
     */
    public void write(BaseDao entity) {
        write(entity, true);
    }

    /**
     * Writes a data object to this transaction which will be commited to the database along with the whole transaction
     */
    public void write(BaseDao entity, boolean validate) {
        if (validate == true) {
            d.dataRepository.validateTrustStructure(entity);
            d.dataRepository.validateTrustPublicKeys(this, entity);
        }

        IPartitionKey partitionKey = entity.partitionKey(true);
        if (written(partitionKey, entity.getId())) {
            return;
        }

        if (validate == true) {
            d.dataRepository.validateReadability(entity);
            d.dataRepository.validateWritability(entity);
        }

        d.debugLogging.logMerge(null, entity, true);

        put(partitionKey, d.currentRights.getRightsWrite());
        put(partitionKey, entity);
    }

    /**
     * Writes a public key to the current transaction and hence eventually to the database
     */
    public void write(IPartitionKey partitionKey, MessagePublicKeyDto key) {
        put(partitionKey, key);
    }

    /**
     * Clears all the partitions that this transaction is tracking
     */
    public void clear() {
        partitions.clear();
        cache.clear();
    }

    /**
     * Flushes all the data records to database
     */
    public void flush(boolean validate, @Nullable DataTransaction copyTo) {
        d.io.send(this, validate);

        if (copyTo != null) {
            copyTo.copyCacheFrom(this);
        }

        if (this.shouldSync) {
            sync();
        }

        clear();
    }

    /**
     * Performs a sync operation on all the partitions that were touched by this data transaction
     */
    public void sync() {
        d.io.sync(this);
    }
}
