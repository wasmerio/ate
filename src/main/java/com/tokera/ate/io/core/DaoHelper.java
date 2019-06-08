package com.tokera.ate.io.core;

import java.lang.reflect.Field;
import java.util.*;

import com.tokera.ate.annotations.ImplicitAuthorityField;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.merge.DataMerger;
import com.tokera.ate.io.repo.DataStagingManager;
import com.tokera.ate.scopes.Startup;
import com.tokera.ate.dao.IParams;
import com.tokera.ate.dao.IRights;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.IRoles;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.units.Secret;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;

/**
 * Helper functions used for common operations on data objects
 */
@Startup
@ApplicationScoped
public class DaoHelper {
    private AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private DataStagingManager staging;
    
    public List<BaseDao> getObjAndParents(BaseDao entity) {
        ArrayList<BaseDao> ret = new ArrayList<>();
        ret.add(entity);
        
        Set<BaseDao> done = new HashSet<>(); 
        done.add(entity);
        
        @Nullable BaseDao parent = this.getParent(entity);
        for (;parent != null; parent = this.getParent(parent)) {
            if (done.contains(parent) == true) break;
            done.add(parent);
            ret.add(parent);
        }
        
        return ret;
    }
    
    public List<BaseDao> getParents(BaseDao entity) {
        ArrayList<BaseDao> ret = new ArrayList<>();
        
        Set<BaseDao> done = new HashSet<>(); 
        
        @Nullable BaseDao parent = this.getParent(entity);
        for (;parent != null; parent = this.getParent(parent)) {
            if (done.contains(parent) == true) break;
            done.add(parent);
            ret.add(parent);
        }
        
        return ret;
    }

    public @Nullable BaseDao getParent(@Nullable BaseDao entity)
    {
        if (entity == null) return null;

        UUID parentId = entity.getParentId();
        if (parentId == null) return null;
        if (parentId.equals(entity.getId())) return null;

        IPartitionKey partitionKey = d.io.partitionResolver().resolve(entity);
        BaseDao ret = this.staging.find(partitionKey, parentId);
        if (ret != null) return ret;

        return d.io.getOrNull(PUUID.from(partitionKey, parentId));
    }

    public @Nullable IParams getDaoParams(PUUID id) {
        BaseDao ret = d.io.getOrNull(id);
        if (ret instanceof IParams) {
            return (IParams)ret;
        }
        return null;
    }

    public @Nullable IRights getDaoRights(PUUID id) {
        BaseDao ret = d.io.getOrNull(id);
        if (ret instanceof IRights) {
            return (IRights)ret;
        }
        return null;
    }

    public @Nullable IRoles getDaoRoles(PUUID id) {
        BaseDao ret = d.io.getOrNull(id);
        if (ret instanceof IRoles) {
            return (IRoles)ret;
        }
        return null;
    }

    private boolean hasImplicitAuthorityInternal(BaseDao entity, String authority) {
        List<Field> fields = DataMerger.getFieldDescriptors(entity.getClass());
        for (Field field : fields) {
            if (field.getAnnotation(ImplicitAuthorityField.class) == null) {
                continue;
            }

            Object val;
            try {
                val = field.get(entity);
                if (val == null) continue;
            } catch (IllegalAccessException e) {
                continue;
            }

            String valStr = val.toString();
            if (authority.equalsIgnoreCase(valStr)) {
                return true;
            }
        }
        return false;
    }

    public boolean hasImplicitAuthority(BaseDao entity, String authority) {
        @Nullable BaseDao obj = entity;
        for (;obj != null; obj = this.getParent(obj)) {
            if (hasImplicitAuthorityInternal(obj, authority)) {
                return true;
            }
        }
        return false;
    }
}
