package com.tokera.ate.dao.base;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.tokera.ate.annotations.Mergable;
import com.tokera.ate.common.CopyOnWrite;
import com.tokera.ate.common.Immutalizable;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.api.IPartitionKeyProvider;
import com.tokera.ate.units.DaoId;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.io.Serializable;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.function.Function;
import java.util.function.Predicate;
import java.util.stream.Collectors;
import java.util.stream.Stream;

/**
 * Represents the common fields and methods of all data objects that are stored in the ATE data-store
 */
@Mergable
public abstract class BaseDao implements Serializable, Immutalizable, IPartitionKeyProvider {

    transient @JsonIgnore @Nullable Set<UUID> _mergesVersions = null;
    transient @JsonIgnore @Nullable UUID _previousVersion = null;
    transient @JsonIgnore boolean _immutable = false;
    transient @JsonIgnore @Nullable IPartitionKey _partitionKey = null;
    transient @JsonIgnore @Nullable String _ioStackTrace = null;

    /**
     * @return Returns the unique primary key of this data entity within the
     * scope of the partition
     */
    @JsonIgnore
    public abstract @DaoId UUID getId();

    /**
     * @return Returns an identifier that can be used to reference this data
     * object even if its in a different partition
     */
    @JsonIgnore
    public PUUID addressableId() {
        return new PUUID(this.partitionKey(true), this.getId());
    }

    /**
     * @return Returns the partition that this object belongs too based on its inheritance tree and the current
     * partition key resolver
     */
    @JsonIgnore
    public IPartitionKey partitionKey() {
        return partitionKey(true);
    }

    /**
     * @return Returns the partition that this object belongs too based on its inheritance tree and the current
     * partition key resolver
     */
    @JsonIgnore
    @Override
    public IPartitionKey partitionKey(boolean shouldThrow) {
        IPartitionKey ret = _partitionKey;
        if (ret != null) return ret;
        if (shouldThrow) {
            ret = AteDelegate.get().io.partitionResolver().resolveOrThrow(this);
        } else {
            ret = AteDelegate.get().io.partitionResolver().resolveOrNull(this);
        }
        _partitionKey = ret;
        return ret;
    }

    /**
     * @return Returns the parent object that this object is attached to
     */
    public abstract @JsonIgnore @Nullable @DaoId UUID getParentId();
    
    @Override
    public int hashCode() {
        int hash = 0;
        hash += getId().hashCode();
        return hash;
    }

    @Override
    public boolean equals(@Nullable Object object) {
        if (object == null) {
            return false;
        }
        if (!(object.getClass() == this.getClass())) {
            return false;
        }
        BaseDao other = (BaseDao) object;
        if (!this.getId().equals(other.getId())) {
            return false;
        }
        return true;
    }

    @Override
    public String toString() {
        return this.getClass().getSimpleName() + "[ id=" + getId().toString().substring(0, 8) + "... ]";
    }

    @Override
    public void immutalize() {
        if (this instanceof CopyOnWrite) {
            ((CopyOnWrite)this).copyOnWrite();
        }
        this._immutable = true;
    }

    boolean hasSaved() {
        if (this._previousVersion != null) return true;
        if (this._mergesVersions != null && this._mergesVersions.size() > 0) return true;
        return false;
    }

    protected void assertStillMutable() {
        assert _immutable == false;
    }

    void pushVersion(UUID previousVersion) {
        _mergesVersions = null;
        _previousVersion = previousVersion;
    }

    public <T extends BaseDao> Stream<T> innerJoin(Class<T> clazz, Function<T, UUID> joiningField) {
        UUID id = getId();
        return AteDelegate.get().io.view(this.partitionKey(), clazz, a -> id.equals(joiningField.apply(a)));
    }

    public <T extends BaseDao> List<T> innerJoinAsList(Class<T> clazz, Function<T, UUID> joiningField) {
        UUID id = getId();
        return AteDelegate.get().io.viewAsList(this.partitionKey(), clazz, a -> id.equals(joiningField.apply(a)));
    }

    public <T extends BaseDao> Set<T> innerJoinAsSet(Class<T> clazz, Function<T, UUID> joiningField) {
        UUID id = getId();
        return AteDelegate.get().io.viewAsSet(this.partitionKey(), clazz, a -> id.equals(joiningField.apply(a)));
    }

    public <T extends BaseDao, K, V> Map<K, V> innerJoinAsMap(Class<T> clazz, Function<T, UUID> joiningField, Function<T, K> mapKey, Function<T, V> mapVal) {
        UUID id = getId();
        return AteDelegate.get().io.viewAsMap(this.partitionKey(), clazz, a -> id.equals(joiningField.apply(a)), mapKey, mapVal);
    }
}