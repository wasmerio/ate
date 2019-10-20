package com.tokera.ate.delegates;

import com.google.common.cache.Cache;
import com.google.common.cache.CacheBuilder;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.enumerations.PermissionPhase;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.DataTransaction;

import javax.enterprise.context.ApplicationScoped;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.locks.Lock;
import java.util.concurrent.locks.ReentrantReadWriteLock;
import java.util.function.Function;
import java.util.stream.Collectors;

@ApplicationScoped
public class IndexingDelegate {
    private final AteDelegate d = AteDelegate.get();
    private final ConcurrentHashMap<TypeKey, Context> contexts = new ConcurrentHashMap<>();

    private class TypeKey{
        private final IPartitionKey partKey;
        private final String otherClazzName;

        private TypeKey(IPartitionKey partKey, String otherClazzName) {
            this.partKey = partKey;
            this.otherClazzName = otherClazzName;
        }

        @Override
        public int hashCode() {
            int hashCode = this.partKey.hashCode();
            hashCode = 37 * hashCode + this.otherClazzName.hashCode();
            return hashCode;
        }

        @Override
        public boolean equals(Object _other) {
            if (_other instanceof TypeKey) {
                TypeKey other = (TypeKey)_other;
                if (partKey.equals(other.partKey) == false) return false;
                if (otherClazzName.equals(other.otherClazzName) == false) return false;
                return true;
            }
            return false;
        }
    }

    private class Context {
        private final TypeKey typeKey;
        private final Cache<IndexKey, IndexKey.Table> tables;

        public Context(TypeKey typeKey) {
            this.typeKey = typeKey;
            this.tables = CacheBuilder.newBuilder()
                    .maximumSize(d.bootstrapConfig.getIndexingMaximumViewsPerTable())
                    .expireAfterAccess(d.bootstrapConfig.getIndexingExpireDelay(), TimeUnit.MILLISECONDS)
                    .build();
        }

        private class IndexKey<T extends BaseDao> {
            private final UUID joiningVal;
            private final Function<T, UUID> joiningKeyMap;
            private final String joiningKeyMapClass;
            private final String rightsHash;
            private final Class<T> otherClazz;

            private IndexKey(Class<T> otherClazz, UUID joiningVal, Function<T, UUID> joiningKeyMap) {
                this.joiningVal = joiningVal;
                this.joiningKeyMap = joiningKeyMap;
                this.joiningKeyMapClass = joiningKeyMap.getClass().getName();
                this.rightsHash = d.currentRights.computeReadRightsHash();
                this.otherClazz = otherClazz;
            }

            @Override
            public int hashCode() {
                int hashCode = this.joiningVal.hashCode();
                hashCode = 37 * hashCode + this.joiningKeyMapClass.hashCode();
                hashCode = 37 * hashCode + this.rightsHash.hashCode();
                return hashCode;
            }

            @Override
            public boolean equals(Object _other) {
                if (_other instanceof IndexKey) {
                    IndexKey other = (IndexKey)_other;
                    if (this.joiningVal.equals(other.joiningVal) == false) return false;
                    if (this.joiningKeyMapClass.equals(other.joiningKeyMapClass) == false) return false;
                    if (this.rightsHash.equals(other.rightsHash) == false) return false;
                    return true;
                }
                return false;
            }

            private class Table {
                private final ReentrantReadWriteLock lock = new ReentrantReadWriteLock();
                private final LinkedHashSet<UUID> values;
                private final LinkedHashSet<UUID> invalidateList;

                private Table() {
                    values = new LinkedHashSet<>(Collections.unmodifiableList(
                            d.io.view(typeKey.partKey, otherClazz, a -> joiningVal.equals(joiningKeyMap.apply(a)))
                                    .map(d -> d.getId())
                                    .collect(Collectors.toList())
                        ));
                    invalidateList = new LinkedHashSet<>();
                }

                @SuppressWarnings("unchecked")
                private List<UUID> fetch() {
                    Lock r = lock.readLock();
                    r.lock();
                    try {
                        if (invalidateList.isEmpty()) {
                            return new ArrayList<>(values);
                        }
                    } finally {
                        r.unlock();
                    }

                    Lock w = lock.writeLock();
                    w.lock();
                    try {
                        for (UUID id : invalidateList) {
                            if (d.io.test(PUUID.from(typeKey.partKey, id), otherClazz, a -> joiningVal.equals(joiningKeyMap.apply(a)))) {
                                if (values.contains(id) == false) {
                                    values.add(id);
                                }
                            } else {
                                values.remove(id);
                            }
                        }
                        invalidateList.clear();
                        return new ArrayList<>(values);
                    } finally {
                        w.unlock();
                    }
                }

                public void invalidate(UUID id) {
                    Lock l = lock.writeLock();
                    l.lock();
                    try {
                        invalidateList.add(id);
                    } finally {
                        l.unlock();
                    }
                }
            }

            private Table createTable() {
                return new Table();
            }
        }

        public void invalidate(UUID id) {
            for (IndexKey.Table table : tables.asMap().values()) {
                table.invalidate(id);
            }
        }

        @SuppressWarnings("unchecked")
        private <T extends BaseDao> IndexKey createIndexKey(Class<T> otherClazz, UUID joiningVal, Function<T, UUID> joiningKeyMap) {
            return new IndexKey(otherClazz, joiningVal, joiningKeyMap);
        }

        public <T extends BaseDao> IndexKey.Table computeTable(IndexKey indexKey) throws ExecutionException {
            return this.tables.get(indexKey, () -> indexKey.createTable());
        }
    }

    public void invalidate(String clazzName, IPartitionKey partKey, UUID id) {
        contexts.compute(new TypeKey(partKey, clazzName), (k, c) -> {
            if (c != null) c.invalidate(id);
            return c;
        });
    }

    private <T extends BaseDao> List<UUID> joinFromTransaction(PUUID id, Class<T> otherClazz, Function<T, UUID> joiningKeyMap, List<UUID> ret) {
        DataTransaction trans = d.io.currentTransaction();
        for (T obj : trans.putsByType(id.partition(), otherClazz)) {
            if (id.id().equals(joiningKeyMap.apply(obj))) {
                ret.add(obj.getId());
            }
        }
        trans.deletes(id.partition()).forEach(a -> ret.remove(a));
        return ret;
    }

    @SuppressWarnings("unchecked")
    public <T extends BaseDao> List<T> join(PUUID id, Class<T> otherClazz, Function<T, UUID> joiningKeyMap) {
        d.requestAccessLog.recordRead(otherClazz);
        if (d.bootstrapConfig.isEnableAutomaticIndexing()) {
            try {
                TypeKey masterKey = new TypeKey(id.partition(), otherClazz.getName());
                Context context = contexts.computeIfAbsent(masterKey, k -> new Context(masterKey));
                List<UUID> ret = context.computeTable(context.createIndexKey(otherClazz, id.id(), joiningKeyMap)).fetch();
                ret = joinFromTransaction(id, otherClazz, joiningKeyMap, ret);
                return d.io.read(id.partition(), ret, otherClazz);
            } catch (ExecutionException e) {
                throw new RuntimeException(e);
            }
        }

        // Otherwise we fall back to table scans
        return d.io.viewAsList(id.partition(), otherClazz, a -> id.id().equals(joiningKeyMap.apply(a)));
    }
}