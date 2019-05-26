package com.tokera.ate.security;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.io.api.IAteIO;
import com.tokera.ate.dao.IRoles;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.msg.MessageDataHeaderDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.dto.EffectivePermissions;
import com.tokera.ate.io.repo.DataContainer;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.Collection;
import java.util.UUID;

/**
 * Builder used to build a list of the effective permissions a data object has in the known tree
 */
public class EffectivePermissionBuilder {

    private final IAteIO ate;
    private IPartitionKey partitionKey;
    private @DaoId UUID origId;
    private @Nullable @DaoId UUID origParentId;
    private boolean usePostMerged = true;

    public EffectivePermissionBuilder(IAteIO ate, PUUID id, @Nullable @DaoId UUID parentId) {
        this.ate = ate;
        this.partitionKey = id;
        this.origId = id.id();
        this.origParentId = parentId;
    }

    public EffectivePermissionBuilder(IAteIO ate, IPartitionKey partitionKey, @DaoId UUID id, @Nullable @DaoId UUID parentId) {
        this.ate = ate;
        this.partitionKey = partitionKey;
        this.origId = id;
        this.origParentId = parentId;
    }

    public EffectivePermissionBuilder setUsePostMerged(boolean val) {
        this.usePostMerged = val;
        return this;
    }

    /**
     * @return Builds a series of lists that represent the permissions a particular data object has in the known tree.
     */
    public EffectivePermissions build()
    {
        return buildWith(null);
    }

    /**
     * @return Builds a series of lists that represent the permissions a particular data object has in the known tree.
     */
    public EffectivePermissions buildWith(@Nullable BaseDao obj)
    {
        EffectivePermissions ret = new EffectivePermissions();
        addRootTrust(ret);
        addChainTrust(ret);
        if (usePostMerged) {
            addPostMergedPerms(ret, obj);
        }
        return ret;
    }

    /**
     * The root of the chain of trust must be added first as this is the most up-to-date key that we can use
     * writing data into the chain which has been accepted into the chain
     */
    private void addRootTrust(EffectivePermissions ret) {
        MessageDataHeaderDto rootOfTrust = ate.getRootOfTrust(PUUID.from(this.partitionKey, this.origId));
        if (rootOfTrust != null) {
            ret.encryptKeyHash = rootOfTrust.getEncryptKeyHash();
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
        @DaoId UUID id = origId;
        @DaoId UUID parentId = origParentId;
        do {
            DataContainer container = ate.getRawOrNull(PUUID.from(this.partitionKey, id));
            if (container != null) {
                MessageDataHeaderDto header = container.getMergedHeader();

                if (ret.encryptKeyHash == null) {
                    ret.encryptKeyHash = header.getEncryptKeyHash();
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

    /**
     * If the user asks for it then we can also add permissions that will be added once the mergeThreeWay
     * into the chain of trust takes place (useful for pre-mergeThreeWay checks)
     */
    private void addPostMergedPerms(EffectivePermissions ret, @Nullable BaseDao retObj) {
        boolean isParents = false;
        boolean inheritRead = true;
        boolean inheritWrite = true;

        @DaoId UUID id = origId;
        @DaoId UUID parentId = origParentId;
        do
        {
            BaseDao obj;
            if (retObj != null && retObj.getId().compareTo(id) == 0) {
                obj = retObj;
            } else {
                obj = ate.getOrNull(PUUID.from(this.partitionKey, id));
            }

            if (obj != null) {
                ret.updateEncryptKeyFromObjIfNull(obj);

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
                    if (roles.getTrustInheritWrite() == false && ate.exists(PUUID.from(this.partitionKey, id)) == true) {
                        inheritWrite = false;
                    }
                }
                parentId = obj.getParentId();
            }
            else
            {
                DataContainer container = ate.getRawOrNull(PUUID.from(this.partitionKey, id));
                if (container != null) {
                    MessageDataHeaderDto header = container.getMergedHeader();

                    if (header.getInheritRead() == false) {
                        inheritRead = false;
                    }
                    if (header.getInheritWrite() == false) {
                        inheritWrite = false;
                    }

                    parentId = header.getParentId();
                }
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
