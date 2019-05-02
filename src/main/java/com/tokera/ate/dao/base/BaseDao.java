package com.tokera.ate.dao.base;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.tokera.ate.common.Immutalizable;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.TopicName;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import java.io.Serializable;
import java.util.Set;
import java.util.UUID;

/**
 * Represents the common fields and methods of all data objects that are stored in the ATE data-store
 */
public abstract class BaseDao implements Serializable, Immutalizable {

    public transient @JsonIgnore @Nullable @TopicName String topicName;
    public transient @JsonIgnore @Nullable Set<UUID> mergesVersions = null;
    public transient @JsonIgnore @Nullable UUID previousVersion = null;
    public transient @JsonIgnore @Nullable UUID version = null;
    protected transient @JsonIgnore boolean _immutable = false;

    /**
     * @return Returns the unique primary key of this data entity within the
     * scope of the topic
     */
    public abstract @DaoId UUID getId();
    
    /**
     * @return Returns the parent object that this object is attached to
     */
    public abstract @Nullable @DaoId UUID getParentId();
    
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

    public boolean hasSaved() {
        return this.version != null;
    }

    @Override
    public void immutalize() {
        this._immutable = true;
    }

    public static void assertStillMutable(@Nullable BaseDao _entity) {
        BaseDao entity = _entity;
        if (entity == null) return;
        assert entity._immutable == false;
    }

    public static void newVersion(BaseDao obj) {
        UUID oldVerison = obj.version;
        obj.version = UUID.randomUUID();
        obj.previousVersion = oldVerison;
    }
}