package com.tokera.ate.security;

import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.dao.IRoles;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.msg.MessageDataHeaderDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.dto.EffectivePermissions;
import com.tokera.ate.io.repo.DataContainer;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.Response;
import java.lang.reflect.Field;
import java.util.Collection;
import java.util.HashMap;
import java.util.Map;
import java.util.UUID;

/**
 * Builder used to build a list of the effective permissions a data object has in the known tree
 */
public class EffectivePermissionBuilder {

    private final AteDelegate d = AteDelegate.get();
    private IPartitionKey partitionKey;
    private @DaoId UUID origId;
    private @Nullable @DaoId UUID origParentId;
    private boolean usePostMerged = true;
    private boolean allowSavingOfChildren = true;
    private final @Nullable Map<UUID, BaseDao> suppliedObjects = new HashMap<>();

    public EffectivePermissionBuilder(PUUID id, @Nullable @DaoId UUID parentId) {
        this.partitionKey = id.partition();
        this.origId = id.id();
        this.origParentId = parentId;
    }

    public EffectivePermissionBuilder(IPartitionKey partitionKey, @DaoId UUID id, @Nullable @DaoId UUID parentId) {
        this.partitionKey = partitionKey;
        this.origId = id;
        this.origParentId = parentId;
    }

    public EffectivePermissionBuilder setUsePostMerged(boolean val) {
        this.usePostMerged = val;
        return this;
    }

    public EffectivePermissionBuilder setAllowSavingOfChildren(boolean val) {
        this.allowSavingOfChildren = val;
        return this;
    }

    /**
     * Supplies a data object that is not yet known to the storage systems
     * @param obj Data object that will be used in the building of the permissions
     */
    public EffectivePermissionBuilder withSuppliedObject(BaseDao obj) {
        this.suppliedObjects.put(obj.getId(), obj);
        return this;
    }

    /**
     * @return Builds a series of lists that represent the permissions a particular data object has in the known tree.
     */
    public EffectivePermissions build()
    {
        EffectivePermissions ret = new EffectivePermissions();
        addRootTrust(ret);
        addChainTrust(ret);
        addImplicitTrust(ret);
        addClaimableTrust(ret);
        if (usePostMerged) {
            addPostMergedPerms(ret);
        }
        return ret;
    }

    /**
     * @return Finds an object based off its ID which either lives in the list of things to be saved or in the actual data store
     */
    public @Nullable BaseDao findDataObj(UUID id) {
        BaseDao obj = MapTools.getOrNull(this.suppliedObjects, id);
        if (obj == null) obj = d.dataStagingManager.find(this.partitionKey, id);
        if (obj == null) obj = d.io.getOrNull(PUUID.from(this.partitionKey, id), false);
        return obj;
    }

    /**
     * The root of the chain of trust must be added first as this is the most up-to-date key that we can use
     * writing data into the chain which has been accepted into the chain
     */
    private void addRootTrust(EffectivePermissions ret) {
        MessageDataHeaderDto rootOfTrust = d.io.getRootOfTrust(PUUID.from(this.partitionKey, this.origId));
        if (rootOfTrust != null) {
            ret.castleId = rootOfTrust.getCastleId();
            ret.rolesRead.addAll(rootOfTrust.getAllowRead());
            ret.rolesWrite.addAll(rootOfTrust.getAllowWrite());
            ret.anchorRolesRead.addAll(rootOfTrust.getAllowRead());
            ret.anchorRolesWrite.addAll(rootOfTrust.getAllowWrite());
        }
    }

    /**
     * Adds all the permissions that come from the chain of trust built for objects in the crypto-graph
     */
    private void addChainTrust(EffectivePermissions ret) {
        boolean inheritRead = true;
        boolean inheritWrite = true;

        // Next we transverse up the tree finding other keys that have already been accepted into the chain of trust
        boolean isFirst = true;
        @DaoId UUID id = origId;
        @DaoId UUID parentId = origParentId;
        do {
            DataContainer container = d.io.getRawOrNull(PUUID.from(this.partitionKey, id));
            if (container != null) {
                MessageDataHeaderDto header = container.getMergedHeader();

                if (isFirst) {
                    ret.castleId = header.getCastleId();
                    isFirst = false;
                }

                if (inheritRead == true) {
                    this.addRolesRead(ret, header.getAllowRead(), true);
                }
                if (inheritWrite == true) {
                    this.addRolesWrite(ret, header.getAllowWrite(), true);
                }
                if (header.getInheritRead() == false) {
                    inheritRead = false;
                }
                if (header.getInheritWrite() == false) {
                    inheritWrite = false;
                }

                parentId = header.getParentId();
            }
            id = parentId;
            parentId = null;
        } while (id != null);
    }

    private void addImplicitTrust(EffectivePermissions ret)
    {
        // Find the object
        DataContainer container = d.io.getRawOrNull(PUUID.from(this.partitionKey, this.origId));
        BaseDao obj = usePostMerged == true ? findDataObj(this.origId) : null;

        // Follow the inheritance tree
        if (container != null) {
            MessageDataHeaderDto header = container.getMergedHeader();
            for (String implicitAuthority : header.getImplicitAuthority()) {
                MessagePublicKeyDto implicitKey = d.implicitSecurity.enquireDomainKey(implicitAuthority, true, container.partitionKey);
                ret.addWriteRole(implicitKey);
            }
        } else if (obj != null) {
            Class<?> type = obj.getClass();
            IPartitionKey key = obj.partitionKey();

            // If it contains dynamic implicit authority
            Field field = MapTools.getOrNull(d.daoParents.getAllowedDynamicImplicitAuthority(), type);
            if (field != null) {
                try {
                    Object domainObj = field.get(obj);
                    if (domainObj == null || domainObj.toString().isEmpty()) {
                        throw new RuntimeException("The implicit authority field can not be null or empty [field: " + field.getName() + "].");
                    }
                    MessagePublicKeyDto implicitKey = d.implicitSecurity.enquireDomainKey(domainObj.toString(), true, key);
                    if (implicitKey == null) {
                        throw new WebApplicationException("No implicit authority found at domain name (missing TXT record)[" + d.bootstrapConfig.getImplicitAuthorityAlias() + "." + domainObj + "].", Response.Status.UNAUTHORIZED);
                    }
                    ret.addWriteRole(implicitKey);
                } catch (IllegalAccessException e) {
                    d.genericLogger.warn(e);
                }
            }

            // If it contains static implicit authority
            String staticImplicitAuthority = MapTools.getOrNull(d.daoParents.getAllowedImplicitAuthority(), type);
            if (staticImplicitAuthority != null) {
                MessagePublicKeyDto implicitKey = d.implicitSecurity.enquireDomainKey(staticImplicitAuthority, true, key);
                ret.addWriteRole(implicitKey);
            }
        }
    }

    private void addClaimableTrust(EffectivePermissions ret) {
        // If the data object is marked as claimable we only add the public write role if the
        // data object does not yet have a root record stored on the chain-of-trust (first come, first serve).
        DataContainer container = d.io.getRawOrNull(PUUID.from(this.partitionKey, this.origId));
        if (container != null)
        {
            if (d.daoParents.getAllowedParentClaimableSimple().contains(container.getPayloadClazz())) {
                MessagePublicKeyDto publicKey = d.encryptor.getTrustOfPublicWrite();
                ret.addWriteRole(publicKey);
            }
        } else if (usePostMerged == true) {
            BaseDao obj = this.findDataObj(this.origId);
            if (obj != null) {
                if (d.daoParents.getAllowedParentClaimable().contains(obj.getClass())) {
                    MessagePublicKeyDto publicKey = d.encryptor.getTrustOfPublicWrite();
                    ret.addWriteRole(publicKey);
                }
            }
        }
    }

    /**
     * If the user asks for it then we can also add permissions that will be added once the mergeThreeWay
     * into the chain of trust takes place (useful for pre-mergeThreeWay checks)
     */
    private void addPostMergedPerms(EffectivePermissions ret) {
        boolean isParents = false;
        boolean inheritRead = true;
        boolean inheritWrite = true;

        @DaoId UUID id = origId;
        @DaoId UUID parentId = origParentId;
        do
        {
            BaseDao obj = this.findDataObj(id);
            if (obj != null) {
                if (obj instanceof IRoles) {
                    IRoles roles = (IRoles) obj;

                    if (inheritRead == true) {
                        addRolesRead(ret, roles.getTrustAllowRead().values(), isParents);
                    }
                    if (inheritWrite == true) {
                        addRolesWrite(ret, roles.getTrustAllowWrite().values(), isParents);
                    }
                    if (roles.getTrustInheritRead() == false) {
                        inheritRead = false;
                    }
                    if (roles.getTrustInheritWrite() == false && d.io.exists(PUUID.from(this.partitionKey, id)) == true) {
                        inheritWrite = false;
                    }
                }
                parentId = obj.getParentId();
            }

            DataContainer container = d.io.getRawOrNull(PUUID.from(this.partitionKey, id));
            if (container != null) {
                MessageDataHeaderDto header = container.getMergedHeader();

                if (inheritRead == true) {
                    addRolesRead(ret, header.getAllowRead(), isParents);
                }
                if (inheritWrite == true) {
                    addRolesWrite(ret, header.getAllowWrite(), isParents);
                }
                if (header.getInheritRead() == false) {
                    inheritRead = false;
                }
                if (header.getInheritWrite() == false) {
                    inheritWrite = false;
                }

                parentId = header.getParentId();
            }

            isParents = true;
            id = parentId;
            parentId = null;
        } while (id != null);
    }

    private void addRolesRead(EffectivePermissions ret, Collection<String> roles, boolean isParents) {
        for (String p : roles) {
            if (ret.rolesRead.contains(p) == false) {
                ret.rolesRead.add(p);
            }
        }
        if (isParents) {
            for (String p : roles) {
                if (ret.anchorRolesRead.contains(p) == false) {
                    ret.anchorRolesRead.add(p);
                }
            }
        }
    }

    private void addRolesWrite(EffectivePermissions ret, Collection<String> roles, boolean isParents) {
        for (String p : roles) {
            if (ret.rolesWrite.contains(p) == false) {
                ret.rolesWrite.add(p);
            }
        }
        if (isParents) {
            for (String p : roles) {
                if (ret.anchorRolesWrite.contains(p) == false) {
                    ret.anchorRolesWrite.add(p);
                }
            }
        }
    }
}
