package com.tokera.ate.io.core;

import java.util.*;

import com.tokera.ate.dao.IParams;
import com.tokera.ate.dao.IRights;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.IRoles;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Secret;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.ApplicationScoped;
import javax.faces.bean.ManagedBean;
import javax.ws.rs.WebApplicationException;

/**
 * Helper functions used for common operations on data objects
 */
@ManagedBean(eager=true)
@ApplicationScoped
public class DaoHelper {
    private AteDelegate d = AteDelegate.getUnsafe();

    public @Secret String generateEncryptKey(IRoles roles)
    {
        int maxFails = 8;
        for (int n = 0;; n++) {
            String key64 = d.encryptor.generateSecret64(128);
            byte[] key = Base64.decodeBase64(key64);

            // Test the key on all the public keys known for it at this time
            // (if any of them fail then we need to try again)
            boolean failed = false;
            Set<MessagePrivateKeyDto> keys = d.currentRights.getRightsRead();
            for (MessagePrivateKeyDto readKey : keys) {
                try {
                    byte[] readPublicBytes = readKey.getPublicKeyBytes();
                    byte[] readPrivateBytes = readKey.getPrivateKeyBytes();
                    if (readPrivateBytes == null || readPublicBytes == null) {
                        if (n > maxFails) {
                            throw new WebApplicationException("Failed to generate an encryption key for entity [clazz=" + roles.getClass().getName() + "] as the private key has no public key bytes.");
                        }
                        failed = true;
                        continue;
                    }
                    byte[] encData = d.encryptor.encryptNtruWithPublic(readPublicBytes, key);
                    byte[] plainData = d.encryptor.decryptNtruWithPrivate(readPrivateBytes, encData);
                    if (!Arrays.equals(key, plainData)) {
                        if (n > maxFails) {
                            throw new WebApplicationException("Failed to generate an encryption key for entity [clazz=" + roles.getClass().getName() + "] validation of the key/pair failed on the encrypt/decrypt test.");
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
                                byte[] bytes = dumpKey.getPublicKeyBytes();
                                if (bytes == null) bytes = new byte[0];

                                msg += "\n" + " - read-key [alias=" + alias + ", hash=" + dumpKey.getPublicKeyHash() + ", size=" + bytes.length + "]";
                            }
                        throw new WebApplicationException(msg, ex);
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

            @Secret String encryptKey = roles.getEncryptKey();
            if (encryptKey == null) {
                if (!shouldCreate) {
                    return null;
                }

                generateEncryptKey(roles);

                encryptKey = roles.getEncryptKey();
                if (encryptKey != null && shouldSave) {
                    d.headIO.mergeLater(entity);
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

        throw new WebApplicationException("Failed to generate an encryption key for entity [clazz=" + entity.getClass().getName() + ", id=" + entity.getId() + "].");
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

        @DaoId UUID parentId = entity.getParentId();
        if (parentId == null) return null;
        if (d.headIO.exists(parentId) == false) return null;

        if (parentId.equals(entity.getId())) return null;
        return d.headIO.getOrNull(parentId);
    }

    public @Nullable IParams getDaoParams(@DaoId UUID id) {
        BaseDao ret = d.headIO.getOrNull(id);
        if (ret instanceof IParams) {
            return (IParams)ret;
        }
        return null;
    }

    public @Nullable IRights getDaoRights(@DaoId UUID id) {
        BaseDao ret = d.headIO.getOrNull(id);
        if (ret instanceof IRights) {
            return (IRights)ret;
        }
        return null;
    }

    public @Nullable IRoles getDaoRoles(@DaoId UUID id) {
        BaseDao ret = d.headIO.getOrNull(id);
        if (ret instanceof IRoles) {
            return (IRoles)ret;
        }
        return null;
    }
}
