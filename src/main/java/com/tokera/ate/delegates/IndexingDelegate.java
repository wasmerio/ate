package com.tokera.ate.delegates;

import com.google.common.cache.Cache;
import com.google.common.cache.CacheBuilder;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.core.PartitionKeyComparator;

import javax.enterprise.context.ApplicationScoped;
import java.lang.ref.WeakReference;
import java.util.Collections;
import java.util.List;
import java.util.UUID;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.function.Function;
import java.util.stream.Collectors;

@ApplicationScoped
public class IndexingDelegate {
    private final AteDelegate d = AteDelegate.get();
    private final PartitionKeyComparator partitionKeyComparator = new PartitionKeyComparator();
    private final Cache<TypeKey, Context> contexts;

    public IndexingDelegate() {
        contexts = CacheBuilder.newBuilder()
                .build();
    }

    private class TypeKey implements Comparable<TypeKey> {
        private final IPartitionKey partKey;
        private final String otherClazzName;

        private TypeKey(IPartitionKey partKey, String otherClazzName) {
            this.partKey = partKey;
            this.otherClazzName = otherClazzName;
        }

        @Override
        public int compareTo(TypeKey other) {
            int diff = partitionKeyComparator.compare(partKey, other.partKey);
            if (diff != 0) return diff;
            return otherClazzName.compareTo(other.otherClazzName);
        }

        @Override
        public int hashCode() {
            int hashCode = this.partKey.hashCode();
            hashCode = 37 * hashCode + this.otherClazzName.hashCode();
            return hashCode;
        }

        @Override
        public boolean equals(Object other) {
            if (other instanceof TypeKey) {
                return compareTo((TypeKey)other) == 0;
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

        private class IndexKey<T extends BaseDao> implements Comparable<IndexKey> {
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
            public int compareTo(IndexKey other) {
                int diff = this.joiningVal.compareTo(other.joiningVal);
                if (diff != 0) return diff;
                diff = joiningKeyMapClass.compareTo(other.joiningKeyMapClass);
                if (diff != 0) return diff;
                return rightsHash.compareTo(other.rightsHash);
            }

            @Override
            public int hashCode() {
                int hashCode = this.joiningVal.hashCode();
                hashCode = 37 * hashCode + this.joiningKeyMapClass.hashCode();
                hashCode = 37 * hashCode + this.rightsHash.hashCode();
                return hashCode;
            }

            @Override
            public boolean equals(Object other) {
                if (other instanceof IndexKey) {
                    return compareTo((IndexKey)other) == 0;
                }
                return false;
            }

            private class Table {
                private volatile WeakReference<List<UUID>> weakJoins;

                private Table() {
                    this.weakJoins = new WeakReference<>(build());
                }

                private List<UUID> build() {
                    return Collections.unmodifiableList(
                            d.io.view(typeKey.partKey, otherClazz, a -> joiningVal.equals(joiningKeyMap.apply(a)))
                            .map(d -> d.getId())
                            .collect(Collectors.toList())
                        );
                }

                @SuppressWarnings("unchecked")
                private List<T> fetch() {
                    List<UUID> joins = this.weakJoins.get();
                    if (joins == null) {
                        joins = build();
                        this.weakJoins = new WeakReference<>(joins);
                    }
                    return d.io.read(typeKey.partKey, joins, otherClazz);
                }
            }

            private Table createTable() {
                return new Table();
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

    public void invalidate(IPartitionKey partKey, String clazzName) {
        contexts.invalidate(new TypeKey(partKey, clazzName));
    }

    @SuppressWarnings("unchecked")
    public <T extends BaseDao> List<T> innerJoin(BaseDao obj, Class<T> otherClazz, Function<T, UUID> joiningKeyMap) {
        if (d.bootstrapConfig.isEnableAutomaticIndexing()) {
            try {
                TypeKey masterKey = new TypeKey(obj.partitionKey(), otherClazz.getName());
                Context context = contexts.get(masterKey, () -> new Context(masterKey));
                return context.computeTable(context.createIndexKey(otherClazz, obj.getId(), joiningKeyMap)).fetch();
            } catch (ExecutionException e) {
                throw new RuntimeException(e);
            }
        }

        // Otherwise we fall back to table scans
        return d.io.viewAsList(obj.partitionKey(), otherClazz, a -> obj.getId().equals(joiningKeyMap.apply(a)));
    }
}
