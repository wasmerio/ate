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

    public @Secret String generateEncryptKey(IRoles roles)
    {
        int maxFails = 8;
        for (int n = 0;; n++) {
            String key64 = d.encryptor.generateSecret64();
            byte[] key = Base64.decodeBase64(key64);

            // Test the key on all the public keys known for it at this time
            // (if any of them fail then we need to try again)
            boolean failed = false;
            Set<MessagePrivateKeyDto> keys = d.currentRights.getRightsRead();
            for (MessagePrivateKeyDto readKey : keys) {
                try {
                    byte[] encData = d.encryptor.encrypt(readKey, key);
                    byte[] plainData = d.encryptor.decrypt(readKey, encData);
                    if (!Arrays.equals(key, plainData)) {
                        if (n > maxFails) {
                            throw new RuntimeException("Failed to generate an encryption key for entity [clazz=" + roles.getClass().getName() + "] validation of the key/pair failed on the encrypt/decrypt test.");
                        }
                        failed = true;
                        continue;
                    }

                    failed = false;

                } catch (Throwable ex) {
                    if (n > maxFails) {
                        String msg = "Failed to generate an encryption key for entity [clazz=" + roles.getClass().getName() + "]";
                            for (MessagePrivateKeyDto dumpKey : keys) {
                                String alias = dumpKey.getAlias();
                                if (alias == null) alias = "null";

                                msg += "\n" + " - read-key [alias=" + alias + ", hash=" + dumpKey.getPublicKeyHash() + "]";
                            }
                        throw new RuntimeException(msg, ex);
                    }
                    failed = true;
                    continue;
                }
            }
            if (failed) continue;

            roles.setEncryptKey(key64);
            return key64;
        }
    }
    
    public @Nullable @Secret String getEncryptKeySingle(BaseDao entity, boolean shouldCreate, boolean shouldSave)
    {
        if (entity instanceof IRoles) {
            IRoles roles = (IRoles)entity;

            // Read the current encryption key set at this node
            @Secret String encryptKey = roles.getEncryptKey();

            // If we have a parent and our read roles are the same as the parent then we shouldn't fork
            // the chain-of-trust here as it will cause performance problems if every node forks so we
            // must merge the trees
            if (entity.getParentId() != null &&
                    roles.getTrustAllowRead().size() == 0 &&
                    roles.getTrustInheritRead() == true)
            {
                if (roles.getEncryptKey() != null && shouldSave) {
                    roles.setEncryptKey(null);
                    d.io.mergeLater(entity);
                }
                return null;
            }

            // Otherwise we need to fork the chain-of-trust however if we are prohibiting that due to some flags
            // then we will fail instead
            if (encryptKey == null) {
                if (!shouldCreate) {
                    return null;
                }

                // Generate an encryption key and save it
                generateEncryptKey(roles);
                encryptKey = roles.getEncryptKey();
                if (encryptKey != null && shouldSave) {
                    d.io.mergeLater(entity);
                }
            }
            return encryptKey;
        }
        return null;
    }
    
    public @Nullable @Secret String getEncryptKey(BaseDao entity, boolean shouldCreate, boolean shouldSave)
    {
        String ret = getEncryptKeySingle(entity, shouldCreate, shouldSave);
        if (ret != null) return ret;
        
        // Check the parents
        for (BaseDao parent : this.getParents(entity))
        {
            ret = getEncryptKeySingle(parent, shouldCreate, shouldSave);
            if (ret != null) return ret;
        }

        if (entity instanceof IRoles) {
            ret = getEncryptKeySingle(entity, shouldCreate, shouldSave);
            if (ret != null) return ret;
        }

        throw new RuntimeException("Failed to generate an encryption key for entity [clazz=" + entity.getClass().getSimpleName() + ", id=" + entity.getId() + "].");
    }
    
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
