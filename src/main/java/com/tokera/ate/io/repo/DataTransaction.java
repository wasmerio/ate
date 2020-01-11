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

    public boolean shouldSync;
    private final Map<IPartitionKey, PartitionContext> partitions = new TreeMap<>(new PartitionKeyComparator());
    private final Map<IPartitionKey, PartitionCache> cache = new TreeMap<>(new PartitionKeyComparator());

    @SuppressWarnings("initialization.fields.uninitialized")
    private DataSubscriber subscriber;

    /**
     * Context of the transaction to a particular partition
     */
    protected class PartitionContext {
        public final List<UUID> toPutOrder = new ArrayList<>();
        public final HashMap<String, LinkedHashSet<UUID>> toPutByType = new HashMap<>();
        public final List<UUID> toDeleteOrder = new ArrayList<>();
        public final Map<UUID, BaseDao> toPut = new HashMap<>();
        public final HashSet<UUID> toDelete = new HashSet<>();
        public final Map<String, MessagePrivateKeyDto> savedWriteKeys = new HashMap<>();
        public final Map<String, MessagePublicKeyDto> savedPublicKeys = new HashMap<>();
        public final Map<UUID, MessageDataDto> savedDatas = new HashMap<>();
        public final HashSet<UUID> savedDeletes = new HashSet<>();
    }

    protected class PartitionCache {
        public final Map<UUID, BaseDao> entries = new HashMap<>();
        public final Map<String, MessagePublicKeyDto> publicKeys = new HashMap<>();
        public final Map<String, MessageSecurityCastleDto> castles = new HashMap<>();
    }

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
            myContext.savedDeletes.addAll(otherContext.savedDeletes);

            for (UUID id : otherContext.savedDatas.keySet()) {
                BaseDao obj = myContext.toPut.remove(id);
                if (obj != null) {
                    myContext.toPutOrder.remove(id);
                    myContext.toPutByType.computeIfPresent(BaseDaoInternal.getType(obj), (k, s) -> {
                        s.remove(id);
                        return s.size() > 0 ? s : null;
                    });
                }
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

    void deleted(IPartitionKey partitionKey, UUID id) {
        PartitionContext context = getPartitionMergeContext(partitionKey, true);
        context.savedDeletes.add(id);
    }

    void put(IPartitionKey partitionKey, BaseDao obj) {
        UUID id = obj.getId();
        String clazzName = BaseDaoInternal.getType(obj);

        PartitionContext context = getPartitionMergeContext(partitionKey, true);
        if (context.toPut.containsKey(id) == false) {
            context.toPut.put(id, obj);
            context.toPutOrder.add(id);
            context.toPutByType.computeIfAbsent(clazzName, k -> new LinkedHashSet<>()).add(id);
        }
        if (context.toDelete.remove(id)) {
            context.toDeleteOrder.remove(id);
        }

        cache(partitionKey, obj);
    }

    void delete(IPartitionKey partitionKey, BaseDao obj) {
        delete(partitionKey, obj.getId());
    }

    public void delete(PUUID pid) {
        delete(pid.partition(), pid.id());
    }

    void delete(IPartitionKey partitionKey, UUID id) {
        d.requestContext.currentTransaction().put(partitionKey,
                d.currentRights.getRightsWrite().stream().map(k -> k.key()).collect(Collectors.toSet()));

        PartitionContext context = getPartitionMergeContext(partitionKey, true);
        if (context.toDelete.contains(id) == false) {
            context.toDelete.add(id);
            context.toDeleteOrder.add(id);
        }
        BaseDao obj = context.toPut.remove(id);
        if (obj != null) {
            context.toPutOrder.remove(id);
            context.toPutByType.computeIfPresent(BaseDaoInternal.getType(obj), (k, s) -> {
                s.remove(id);
                return s.size() > 0 ? s : null;
            });

            String clazzName = BaseDaoInternal.getType(obj);
        }

        uncache(partitionKey, id);
    }

    void undo(IPartitionKey partitionKey, BaseDao obj) {
        UUID id = obj.getId();
        PartitionContext context = getPartitionMergeContext(partitionKey, true);
        if (context.toDelete.remove(id)) {
            context.toDeleteOrder.remove(id);
        }

        context.toPut.remove(id);
        context.toPutOrder.remove(id);
        context.toPutByType.computeIfAbsent(BaseDaoInternal.getType(obj), k -> new LinkedHashSet<>()).add(id);

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
        BaseDao obj = c.entries.remove(id);
        return obj != null;
    }

    public Collection<IPartitionKey> keys() {
        return this.partitions.keySet();
    }

    public Collection<IPartitionKey> allKeys() {
        Set<IPartitionKey> ret = new HashSet<>();
        ret.addAll(this.partitions.keySet());
        ret.addAll(this.cache.keySet());
        return ret;
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
        if (context == null) return Collections.emptyList();
        return context.toPutOrder.stream().map(id -> context.toPut.get(id)).collect(Collectors.toList());
    }

    @SuppressWarnings("unchecked")
    public <T extends BaseDao> Iterable<T> putsByType(IPartitionKey partitionKey, Class<T> clazz) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return Collections.emptyList();
        LinkedHashSet<UUID> list = MapTools.getOrNull(context.toPutByType, clazz.getName());
        if (list == null) return Collections.emptyList();
        return list.stream()
                .map(id -> (T)context.toPut.get(id))
                .collect(Collectors.toList());
    }

    public Iterable<BaseDao> putsByType(IPartitionKey partitionKey, String clazz) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return Collections.emptyList();
        LinkedHashSet<UUID> list = MapTools.getOrNull(context.toPutByType, clazz);
        if (list == null) return Collections.emptyList();
        return list.stream()
                .map(id -> context.toPut.get(id))
                .collect(Collectors.toList());
    }

    public Iterable<BaseDao> putsByPartition(IPartitionKey partitionKey) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return Collections.emptyList();
        return context.toPut.values().stream()
                .collect(Collectors.toList());
    }

    public Iterable<UUID> deletes(IPartitionKey partitionKey) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return Collections.emptyList();
        return context.toDeleteOrder.stream().collect(Collectors.toList());
    }

    public boolean isWritten(PUUID id) {
        return this.isWritten(id.partition(), id.id());
    }

    public boolean isWritten(IPartitionKey partitionKey, UUID id)
    {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return false;
        return context.toPut.containsKey(id);
    }

    public boolean isWrittenOrSaved(PUUID id) {
        return this.isWrittenOrSaved(id.partition(), id.id());
    }

    public boolean isWrittenOrSaved(IPartitionKey partitionKey, UUID id)
    {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return false;
        return context.toPut.containsKey(id) || context.savedDatas.containsKey(id);
    }

    public boolean isDeleted(PUUID id) {
        return isDeleted(id.partition(), id.id());
    }

    public boolean isDeleted(IPartitionKey partitionKey, UUID id) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return false;
        return context.toDelete.contains(id);
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
        MessagePublicKeyDto ret = d.currentRights.findKeyAndConvertToPublic(publicKeyHash);
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

        return Collections.emptyList();
    }

    public @Nullable MessagePrivateKeyDto findPrivateKey(IPartitionKey partitionKey, String publicKeyHash) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return null;
        return MapTools.getOrNull(context.savedWriteKeys, publicKeyHash);
    }

    Collection<MessagePrivateKeyDto> findPrivateKeys(IPartitionKey partitionKey) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return Collections.emptyList();
        return context.savedWriteKeys.values();
    }

    public @Nullable MessageDataDto findSavedData(IPartitionKey partitionKey, UUID id) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return null;
        return MapTools.getOrNull(context.savedDatas, id);
    }
    public boolean findSavedDelete(IPartitionKey partitionKey, UUID id) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return false;
        return context.savedDeletes.contains(id);
    }

    public @Nullable MessagePublicKeyDto findSavedPublicKey(IPartitionKey partitionKey, String hash) {
        PartitionContext context = getPartitionMergeContext(partitionKey, false);
        if (context == null) return null;
        return MapTools.getOrNull(context.savedPublicKeys, hash);
    }

    public Map<UUID, MessageDataDto> getSavedDataMap(IPartitionKey partitionKey) {
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
        if (isWritten(partitionKey, entity.getId())) {
            return;
        }

        if (validate == true) {
            d.dataRepository.validateReadability(entity);
            d.dataRepository.validateWritability(entity);
        }

        d.debugLogging.logMerge(null, entity, true);

        put(partitionKey, d.currentRights.getRightsWrite().stream().map(k -> k.key()).collect(Collectors.toSet()));
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
            d.transaction.finish();
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
