/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.io.repo;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.base.BaseDaoInternal;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageDataHeaderDto;
import com.tokera.ate.dto.msg.MessageDataMetaDto;
import com.tokera.ate.dto.msg.MessageMetaDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.merge.MergePair;
import org.checkerframework.checker.nullness.qual.NonNull;
import org.checkerframework.checker.nullness.qual.Nullable;
import sun.reflect.generics.tree.Tree;

import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentSkipListSet;
import java.util.concurrent.locks.Lock;
import java.util.concurrent.locks.ReentrantReadWriteLock;
import java.util.function.Predicate;
import java.util.stream.Collectors;

public class DataContainer {
    private final AteDelegate d = AteDelegate.get();

    public final UUID id;
    public final IPartitionKey partitionKey;

    private Long firstOffset = 0L;
    private Long lastOffset = 0L;

    private final Map<UUID, @NonNull DataGraphNode> lookup = new HashMap<>();
    private final TreeMap<Long, DataGraphNode> timeline = new TreeMap<>();
    private final TreeMap<Long, DataGraphNode> leafs = new TreeMap<>();
    private final ConcurrentSkipListSet<String> keys = new ConcurrentSkipListSet<>();
    private @Nullable String leafKey = null;
    private @Nullable UUID leafCastleId = null;
    private final ReentrantReadWriteLock lock = new ReentrantReadWriteLock();

    private volatile @Nullable DataContainer parent = null;
    private final LinkedHashMap<UUID, DataContainer> children = new LinkedHashMap<>();

    // These objects allow for much faster access of the data
    private BaseDao cacheObj;
    private HashSet<String> cacheOwners = new HashSet<>();

    public DataContainer(UUID id, IPartitionKey partitionKey) {
        this.id = id;
        this.partitionKey = partitionKey;
    }

    private DataContainer add(MessageDataMetaDto msg) {
        UUID castleId = msg.getHeader().getCastleId();
        String key = msg.getMeta().getKey();

        DataGraphNode node = new DataGraphNode(msg);
        Lock w = this.lock.writeLock();
        w.lock();
        try {
            // Update the offset range
            long offset = msg.getMeta().getOffset();
            if (firstOffset == 0L || offset < firstOffset) {
                firstOffset = offset;
            }
            if (offset > lastOffset) {
                lastOffset = offset;
            }

            // If the compacted topic key changes then the chain-of-versions needs to be reset
            // as otherwise previous compacted keys will get merged into the last key
            boolean addToLeaf = true;
            if (key.equals(leafKey) == false ||
                castleId.equals(leafCastleId) == false)
            {
                // Only rows that relate to the future impact the timeline
                if (offset > leafs.keySet().stream().max(Long::compare).orElse(0L)) {
                    leafs.clear();
                    leafKey = key;
                    leafCastleId = castleId;
                }
                // Otherwise we need to ignore this row as its on the wrong key and in the past
                else {
                    addToLeaf = false;
                }
            }

            // Empty payloads should not attempt a merge
            if (msg.hasPayload()) {
                for (DataGraphNode removeNode : leafs.values().stream()
                        .filter(n -> n.version.equals(node.previousVersion) ||
                                     node.mergesVersions.contains(n.version))
                        .collect(Collectors.toList()))
                {
                    leafs.remove(removeNode.msg.getMeta().getOffset(), removeNode);
                }
            } else {
                leafs.clear();
            }

            // Only if this is a relavent row do we add it to the leaf
            if (addToLeaf == true) {
                leafs.put(offset, node);
            }

            // Add the node to the merge list and immutalize it
            lookup.put(node.version, node);
            timeline.put(offset, node);
            keys.add(node.key);
            msg.immutalize();

            // Clear the cache and leave the lock
            cacheObj = null;
            cacheOwners.clear();
        } finally {
            w.unlock();
        }
        return this;
    }

    public boolean requiresMerge() {
        Lock w = this.lock.readLock();
        w.lock();
        try {
            return leafs.size() > 1;
        } finally {
            w.unlock();
        }
    }

    public void clear() {
        Lock w = this.lock.writeLock();
        w.lock();
        try {
            leafs.clear();;
            timeline.clear();
            lookup.clear();
            cacheObj = null;
            cacheOwners.clear();
        } finally {
            w.unlock();
        }
    }

    public boolean isEmpty() {
        Lock w = this.lock.readLock();
        w.lock();
        try {
            return timeline.isEmpty();
        } finally {
            w.unlock();
        }
    }

    public boolean hasVersion(UUID version) {
        Lock w = this.lock.readLock();
        w.lock();
        try {
            return this.lookup.containsKey(version);
        } finally {
            w.unlock();
        }
    }

    public List<DataGraphNode> timeline() {
        List<DataGraphNode> ret = new ArrayList<>();
        Lock w = this.lock.readLock();
        w.lock();
        try {
            ret.addAll(timeline.values());
        } finally {
            w.unlock();
        }
        return ret;
    }

    public Set<String> keys() {
        return this.keys;
    }

    public DataContainer add(MessageDataDto data, MessageMetaDto meta) {
        MessageDataMetaDto msg = new MessageDataMetaDto(data, meta);
        msg.immutalize();
        this.add(msg);
        return this;
    }

    public @Nullable MessageDataMetaDto getLastOrNull() {
        Lock r = this.lock.readLock();
        r.lock();
        try {
            if (timeline.size() <= 0) return null;
            return timeline.lastEntry().getValue().msg;
        } finally {
            r.unlock();
        }
    }

    public @Nullable MessageDataHeaderDto getLastHeaderOrNull() {
        MessageDataMetaDto last = getLastOrNull();
        if (last == null) return null;
        return last.getData().getHeader();
    }

    public @Nullable Long getLastOffsetOrNull() {
        MessageDataMetaDto last = getLastOrNull();
        if (last == null) return null;
        return last.getMeta().getOffset();
    }

    public @Nullable MessageDataDto getLastDataOrNull() {
        MessageDataMetaDto last = getLastOrNull();
        if (last == null) return null;
        return last.getData();
    }

    public String getPayloadClazz() {
        MessageDataHeaderDto lastHeader = getLastHeaderOrNull();
        if (lastHeader == null) return "[null]";
        return lastHeader.getPayloadClazzOrThrow();
    }

    public String getPayloadClazzShort() {
        MessageDataHeaderDto lastHeader = getLastHeaderOrNull();
        if (lastHeader == null) return "[null]";
        return lastHeader.getPayloadClazzShortOrThrow();
    }

    public @Nullable UUID getParentId() {
        MessageDataHeaderDto lastHeader = getLastHeaderOrNull();
        if (lastHeader == null) return null;
        return lastHeader.getParentId();
    }

    public Long getFirstOffset() {
        Lock r = this.lock.readLock();
        r.lock();
        try {
            return this.firstOffset;
        } finally {
            r.unlock();
        }
    }

    public Long getLastOffset() {
        Lock r = this.lock.readLock();
        r.lock();
        try {
            return this.lastOffset;
        } finally {
            r.unlock();
        }
    }

    public boolean getImmutable() {
        MessageDataHeaderDto lastHeader = getLastHeaderOrNull();
        if (lastHeader == null) return false;
        return lastHeader.getInheritWrite() == false && lastHeader.getAllowWrite().isEmpty();
    }

    public boolean hasPayload() {
        MessageDataMetaDto last = getLastOrNull();
        if (last == null) return false;
        return last.getData().hasPayload();
    }

    public Iterable<MessageMetaDto> getHistory() {
        Lock r = this.lock.readLock();
        r.lock();
        try {
            return this.timeline.values().stream()
                    .map(a -> a.msg.getMeta())
                    .collect(Collectors.toList());
        } finally {
            r.unlock();
        }
    }

    private @Nullable LinkedList<DataGraphNode> computeCurrentLeaves() {
        Lock r = this.lock.readLock();
        r.lock();
        try {
            if (this.leafs.isEmpty()) return null;

            HashSet<UUID> ignoreThese = new HashSet<>();

            LinkedList<DataGraphNode> ret = new LinkedList<>();
            for (Map.Entry<Long, DataGraphNode> entry : this.leafs.descendingMap().entrySet()) {
                DataGraphNode node = entry.getValue();
                if (node.msg.getData().hasPayload() == false) break;

                if (node.previousVersion != null) {
                    ignoreThese.add(node.previousVersion);
                }
                if (node.mergesVersions != null) {
                    ignoreThese.addAll(node.mergesVersions);
                }

                if (ignoreThese.contains(node.version)) continue;
                ignoreThese.add(node.version);

                ret.addFirst(node);
            }
            return ret;
        } finally {
            r.unlock();
        }
    }

    public @Nullable MessageDataHeaderDto getMergedHeader() {
        LinkedList<DataGraphNode> leaves = computeCurrentLeaves();
        if (leaves == null || leaves.isEmpty()) return null;

        MessageDataHeaderDto ret;

        // If there is only one item then we are done
        if (leaves.size() == 1) {
            ret = (MessageDataHeaderDto)d.merger.cloneObject(leaves.get(0).msg.getData().getHeader());
            ret.setPreviousVersion(ret.getVersion());
            ret.setVersion(UUID.randomUUID());
            ret.setMerges(null);
            return ret;
        }

        // Build a merge set of the headers for this
        ArrayList<MergePair<MessageDataHeaderDto>> mergeSet = new ArrayList<>();
        leaves.stream().map(n -> new MergePair<>(
                (n.parentNode != null ? n.parentNode.msg.getData().getHeader() : null),
                n.msg.getData().getHeader()))
            .forEach(a -> mergeSet.add(a));

        // Return the result of the merge
        ret = d.merger.merge(mergeSet);
        if (ret == null) return null;

        // Determine the merge set that was used for this object
        Set<UUID> mergeVersions = mergeSet
                .stream()
                .map(h -> h.what.getVersionOrThrow())
                .collect(Collectors.toSet());

        ret.setPreviousVersion(null);
        ret.setVersion(UUID.randomUUID());
        ret.getMerges().copyFrom(mergeVersions);
        return ret;
    }

    private static @Nullable BaseDao reconcileMergedData(IPartitionKey partitionKey, @Nullable BaseDao _ret, TreeMap<Long, DataGraphNode> leaves) {
        BaseDao ret = _ret;
        if (ret == null) return null;

        // Set the partition key so that it does not attempt to transverse up the tree
        BaseDaoInternal.setPartitionKey(ret, partitionKey);

        // Reconcile the parent version pointers
        if (leaves.size() == 1) {
            BaseDaoInternal.setPreviousVersion(ret, leaves.lastEntry().getValue().version);
            BaseDaoInternal.setMergesVersions(ret, null);
        } else {
            BaseDaoInternal.setPreviousVersion(ret, null);
            BaseDaoInternal.setMergesVersions(ret, leaves.values().stream().map(n -> n.version).collect(Collectors.toSet()));
        }

        return ret;
    }

    @SuppressWarnings("unchecked")
    public <T extends BaseDao> boolean test(Predicate<T> predicate, boolean shouldThrow) {
        BaseDao orig = fetchData(shouldThrow);
        if (orig == null) return false;
        return predicate.test((T)orig);
    }

    private @Nullable BaseDao cloneDataUnderLock(BaseDao orig) {
        return reconcileMergedData(this.partitionKey, d.io.clone(orig), leafs);
    }

    public @Nullable BaseDao fetchData() {
        return this.fetchData(true);
    }

    public @Nullable BaseDao fetchData(boolean shouldThrow) {
        Lock r = this.lock.readLock();
        r.lock();
        try {
            if (this.cacheObj != null) {
                for (PrivateKeyWithSeedDto key : d.currentRights.getRightsRead()) {
                    if (cacheOwners.contains(key.privateHash())) {
                        return cloneDataUnderLock(this.cacheObj);
                    }
                }
            }

            if (timeline.size() <= 0) return null;
            if (timeline.lastEntry().getValue().msg.hasPayload() == false) return null;
        } finally {
            r.unlock();
        }

        Lock w = this.lock.writeLock();
        w.lock();
        try {
            if (this.cacheObj != null) {
                for (PrivateKeyWithSeedDto key : d.currentRights.getRightsRead()) {
                    if (cacheOwners.contains(key.privateHash())) {
                        return cloneDataUnderLock(this.cacheObj);
                    }
                }
            }

            if (timeline.size() <= 0) return null;
            if (timeline.lastEntry().getValue().msg.hasPayload() == false) return null;

            BaseDao ret = createData(shouldThrow);
            if (ret == null) return null;

            MessageDataMetaDto meta = timeline.lastEntry().getValue().msg;
            MessageDataHeaderDto header = meta.getHeader();

            for (PrivateKeyWithSeedDto key : d.currentRights.getRightsRead()) {
                for (String hash : header.getAllowRead()) {
                    if (hash.equals(key.publicHash())) {
                        cacheOwners.add(key.privateHash());
                    }
                }
            }

            ret.immutalize();
            if (this.cacheObj == null) {
                this.cacheObj = ret;
            }

            return cloneDataUnderLock(ret);
        } finally {
            w.unlock();
        }
    }

    @SuppressWarnings("return.type.incompatible")
    private @Nullable BaseDao createData(boolean shouldThrow) {
        LinkedList<DataGraphNode> leaves = computeCurrentLeaves();
        if (leaves == null || leaves.isEmpty()) return null;

        // If there is only one item then we are done
        if (leaves.size() == 1) {
            MessageDataMetaDto msg = leaves.get(0).msg;
            return d.dataSerializer.fromDataMessage(this.partitionKey, msg, shouldThrow);
        }

        // Build a merge set of the headers for this
        Map<DataGraphNode, BaseDao> deserializeCache = new HashMap<>();
        List<MergePair<BaseDao>> mergeSet = leaves
                .stream().map(n -> {
                    BaseDao a = null;
                    if (n.parentNode != null) {
                        a = deserializeCache.computeIfAbsent(n.parentNode, v -> d.dataSerializer.fromDataMessage(this.partitionKey, v.msg, shouldThrow));
                    }
                    BaseDao b = deserializeCache.computeIfAbsent(n, v -> d.dataSerializer.fromDataMessage(this.partitionKey, n.msg, shouldThrow));
                    return new MergePair<>(a, b);
                })
                .collect(Collectors.toList());

        // Merge the actual merge of the data object
        return d.merger.merge(mergeSet);
    }

    public void addChildContainer(DataContainer childContainer)
    {
        Lock w = this.lock.writeLock();
        w.lock();
        try {
            this.children.put(childContainer.id, childContainer);
        } finally {
            w.unlock();
        }
    }

    public void removeChildContainer(DataContainer childContainer)
    {
        Lock w = this.lock.writeLock();
        w.lock();
        try {
            this.children.remove(childContainer.id, childContainer);
        } finally {
            w.unlock();
        }
    }

    public List<DataContainer> getChildContainers() {
        Lock w = this.lock.readLock();
        w.lock();
        try {
            return this.children.values().stream()
                    .collect(Collectors.toList());
        } finally {
            w.unlock();
        }
    }

    public void setParentContainer(DataContainer parentContainer) {
        this.parent = parentContainer;
    }

    public DataContainer getParentContainer() {
        return this.parent;
    }
}