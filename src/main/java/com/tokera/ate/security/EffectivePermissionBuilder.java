package com.tokera.ate.security;

import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDaoInternal;
import com.tokera.ate.dao.enumerations.PermissionPhase;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageDataDto;
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
    private @Nullable String type;
    private IPartitionKey partitionKey;
    private @DaoId UUID origId;
    private PermissionPhase origPhase = PermissionPhase.DynamicStaging;
    private final @Nullable Map<UUID, BaseDao> suppliedObjects = new HashMap<>();

    public EffectivePermissionBuilder(@Nullable String type, PUUID id) {
        this.type = type;
        this.partitionKey = id.partition();
        this.origId = id.id();
    }

    public EffectivePermissionBuilder(@Nullable String type, IPartitionKey partitionKey, @DaoId UUID id) {
        this.type = type;
        this.partitionKey = partitionKey;
        this.origId = id;
    }

    public EffectivePermissionBuilder withPhase(PermissionPhase phase) {
        this.origPhase = phase;
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
     * Compute what phase we should assume this object to be under based on if its been pushed to the chain yet or not
     */
    private PermissionPhase computePhase(UUID id) {
        switch (origPhase) {
            case DynamicStaging:
                return d.requestContext.currentTransaction()
                        .written(this.partitionKey, id)
                        ? PermissionPhase.AfterMerge
                        : PermissionPhase.BeforeMerge;
            case DynamicChain:
                return d.requestContext.currentTransaction()
                       .findSavedData(this.partitionKey, id) != null
                       ? PermissionPhase.AfterMerge
                       : PermissionPhase.BeforeMerge;
            default:
                return origPhase;
        }
    }

    /**
     * @return Builds a series of lists that represent the permissions a particular data object has in the known tree.
     */
    public EffectivePermissions build()
    {
        EffectivePermissions ret = new EffectivePermissions(this.type, this.partitionKey, this.origId);
        reconcileType(ret);

        if (computePhase(origId) == PermissionPhase.BeforeMerge) {
            addRootTrust(ret);
        }

        addChainTrust(ret);

        if (computePhase(origId) == PermissionPhase.BeforeMerge) {
            addImplicitTrust(ret);
            addClaimableTrust(ret);
        }
        return ret;
    }

    /**
     * Reconcile type type name
     */
    private void reconcileType(EffectivePermissions ret) {
        if (ret.type == null) {
            DataContainer container = d.io.readRawOrNull(PUUID.from(this.partitionKey, this.origId));
            if (container != null) ret.type = container.getPayloadClazz();
        }
        if (ret.type == null && computePhase(this.origId) == PermissionPhase.AfterMerge) {
            BaseDao obj = this.findDataObj(this.origId);
            if (obj != null) ret.type = BaseDaoInternal.getType(obj);
        }
    }

    /**
     * @return Finds an object based off its ID which either lives in the list of things to be saved or in the actual data store
     */
    public @Nullable BaseDao findDataObj(UUID id) {
        BaseDao obj = MapTools.getOrNull(this.suppliedObjects, id);
        if (obj == null) obj = d.requestContext.currentTransaction().find(this.partitionKey, id);
        if (obj == null) obj = d.io.readOrNull(PUUID.from(this.partitionKey, id), false);
        return obj;
    }

    /**
     * The root of the chain of trust must be added first as this is the most up-to-date key that we can use
     * writing data into the chain which has been accepted into the chain
     */
    private void addRootTrust(EffectivePermissions ret) {
        MessageDataHeaderDto rootOfTrust = d.io.readRootOfTrust(PUUID.from(this.partitionKey, this.origId));
        if (rootOfTrust != null) {
            ret.castleId = rootOfTrust.getCastleId();
            ret.rolesRead.addAll(rootOfTrust.getAllowRead());
            ret.rolesWrite.addAll(rootOfTrust.getAllowWrite());
        }
    }

    /**
     * Adds all the permissions that come from the chain of trust built for objects in the crypto-graph
     */
    private void addChainTrust(EffectivePermissions ret) {
        boolean inheritRead = true;
        boolean inheritWrite = true;

        boolean isFirst = true;
        @DaoId UUID id = origId;

        for (;id != null;)
        {
            if (computePhase(id) == PermissionPhase.AfterMerge)
            {
                MessageDataDto data = d.requestContext.currentTransaction().findSavedData(partitionKey, id);
                if (data != null) {
                    MessageDataHeaderDto header = data.getHeader();

                    if (isFirst) {
                        ret.castleId = header.getCastleId();
                        isFirst = false;
                    }

                    if (inheritRead == true) {
                        addRolesRead(ret, header.getAllowRead());
                    }
                    if (inheritWrite == true) {
                        addRolesWrite(ret, header.getAllowWrite());
                    }
                    if (header.getInheritRead() == false) {
                        inheritRead = false;
                    }
                    if (header.getInheritWrite() == false) {
                        inheritWrite = false;
                    }

                    id = header.getParentId();
                    continue;
                }

                BaseDao obj = this.findDataObj(id);
                if (obj != null) {
                    if (obj instanceof IRoles) {
                        IRoles roles = (IRoles) obj;

                        if (inheritRead == true) {
                            addRolesRead(ret, roles.getTrustAllowRead().values());
                        }
                        if (inheritWrite == true) {
                            addRolesWrite(ret, roles.getTrustAllowWrite().values());
                        }
                        if (roles.getTrustInheritRead() == false) {
                            inheritRead = false;
                        }
                        if (roles.getTrustInheritWrite() == false) {
                            inheritWrite = false;
                        }
                    }

                    id = obj.getParentId();
                    continue;
                }
            }

            DataContainer container = d.io.readRawOrNull(PUUID.from(this.partitionKey, id));
            if (container != null) {
                MessageDataHeaderDto header = container.getMergedHeader();

                if (isFirst) {
                    ret.castleId = header.getCastleId();
                    isFirst = false;
                }

                if (inheritRead == true) {
                    addRolesRead(ret, header.getAllowRead());
                }
                if (inheritWrite == true) {
                    addRolesWrite(ret, header.getAllowWrite());
                }
                if (header.getInheritRead() == false) {
                    inheritRead = false;
                }
                if (header.getInheritWrite() == false) {
                    inheritWrite = false;
                }

                id = header.getParentId();
                continue;
            }

            MessageDataDto data = d.requestContext.currentTransaction().findSavedData(partitionKey, id);
            if (data != null) {
                id = data.getHeader().getParentId();
                continue;
            }

            BaseDao obj = this.findDataObj(id);
            if (obj != null) {
                id = obj.getParentId();
                continue;
            }
            break;
        }
    }

    private void addImplicitTrust(EffectivePermissions ret)
    {
        // If its already in the chain-of-trust then we just use this ones implicit authority
        DataContainer container = d.io.readRawOrNull(PUUID.from(this.partitionKey, this.origId));
        if (container != null) {
            MessageDataHeaderDto header = container.getMergedHeader();
            for (String implicitAuthority : header.getImplicitAuthority()) {
                MessagePublicKeyDto implicitKey = d.implicitSecurity.enquireDomainKey(implicitAuthority, true, container.partitionKey);
                ret.addWriteRole(implicitKey);
            }
            return;
        }

        // Maybe its been pushed to the chain of trust already
        MessageDataDto data = d.requestContext.currentTransaction().findSavedData(partitionKey, this.origId);
        if (data != null) {
            MessageDataHeaderDto header = container.getMergedHeader();
            for (String implicitAuthority : header.getImplicitAuthority()) {
                MessagePublicKeyDto implicitKey = d.implicitSecurity.enquireDomainKey(implicitAuthority, true, container.partitionKey);
                ret.addWriteRole(implicitKey);
            }
            return;
        }

        // Find the object in the tree
        BaseDao obj = findDataObj(this.origId);
        if (obj != null) {
            Class<?> type = obj.getClass();
            IPartitionKey key = obj.partitionKey(true);

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
            return;
        }
    }

    private void addClaimableTrust(EffectivePermissions ret)
    {
        // If the data object is marked as claimable we only add the public write role if the
        // data object does not yet have a root record stored on the chain-of-trust (first come, first serve).
        if (ret.type != null && d.io.readRawOrNull(PUUID.from(this.partitionKey, this.origId)) == null)  {
            if (d.daoParents.getAllowedParentClaimableSimple().contains(ret.type)) {
                MessagePublicKeyDto publicKey = d.encryptor.getTrustOfPublicWrite();
                ret.addWriteRole(publicKey);
            }
        }
    }

    private void addRolesRead(EffectivePermissions ret, Collection<String> roles) {
        for (String p : roles) {
            if (ret.rolesRead.contains(p) == false) {
                ret.rolesRead.add(p);
            }
        }
    }

    private void addRolesWrite(EffectivePermissions ret, Collection<String> roles) {
        for (String p : roles) {
            if (ret.rolesWrite.contains(p) == false) {
                ret.rolesWrite.add(p);
            }
        }
    }

    public String getType() {
        return type;
    }
}
