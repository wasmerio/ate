package com.tokera.ate.delegates;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.events.RightsValidationEvent;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.scopes.Startup;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.dao.IRights;
import com.tokera.ate.dao.IRoles;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.events.NewAccessRightsEvent;
import com.tokera.ate.io.repo.DataContainer;
import com.tokera.ate.security.EffectivePermissionBuilder;
import com.tokera.ate.dto.EffectivePermissions;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.events.TokenScopeChangedEvent;
import com.tokera.ate.events.TokenStateChangedEvent;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;
import com.tokera.ate.units.Secret;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.Response;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.stream.Collectors;

/**
 * Delegate used to check authorization rights in the currentRights context and scopes
 */
@Startup
@ApplicationScoped
public class AuthorizationDelegate {
    private AteDelegate d = AteDelegate.get();

    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;

    public boolean canRead(@Nullable BaseDao obj)
    {
        if (obj == null) return false;
        EffectivePermissions perms = d.authorization.perms(obj);
        return perms.canRead(d.currentRights);
    }

    public boolean canRead(@Nullable PUUID _id, @Nullable @DaoId UUID parentId)
    {
        PUUID id = _id;
        if (id == null) return false;
        return canRead(id.partition(), id.id(), parentId);
    }

    public boolean canRead(IPartitionKey partitionKey, @DaoId UUID id, @Nullable @DaoId UUID parentId)
    {
        // If its in the cache then we can obviously read it
        if (d.memoryRequestCacheIO.exists(PUUID.from(partitionKey, id)) == true) return true;

        // Otherwise we need to compute some permissions for it
        EffectivePermissions perms = d.authorization.perms(partitionKey, id, parentId, true);
        return perms.canRead(d.currentRights);
    }

    public boolean canWrite(@Nullable BaseDao obj)
    {
        if (obj == null) return false;
        EffectivePermissions perms = d.authorization.perms(obj);
        return perms.canWrite(d.currentRights);
    }

    public boolean canWrite(@Nullable PUUID _id, @Nullable @DaoId UUID parentId)
    {
        PUUID id = _id;
        if (id == null) return false;
        return canWrite(id.partition(), id.id(), parentId);
    }

    public void ensureCanWrite(BaseDao obj)
    {
        if (canWrite(obj) == false) {
            IPartitionKey partitionKey = obj.partitionKey();
            EffectivePermissions permissions = d.authorization.perms(obj);
            throw buildWriteException(partitionKey, obj.getId(), permissions, true);
        }
    }

    public boolean canWrite(IPartitionKey partitionKey, @DaoId UUID id, @Nullable @DaoId UUID parentId)
    {
        EffectivePermissions perms = d.authorization.perms(partitionKey, id, parentId, true);
        return perms.canWrite(d.currentRights);
    }

    public RuntimeException buildWriteException(IPartitionKey partitionKey, @DaoId UUID entityId, EffectivePermissions permissions, boolean showStack)
    {
        StringBuilder sb = new StringBuilder();
        sb.append("Access denied while attempting to write object [");
        DataContainer container = d.io.getRawOrNull(PUUID.from(partitionKey, entityId));
        if (container != null) {
            sb.append(container.getPayloadClazz()).append(":");
        } else {
            BaseDao obj = d.dataStagingManager.find(PUUID.from(partitionKey, entityId));
            if (obj != null) {
                sb.append(obj.getClass().getSimpleName()).append(":");
            }
        }
        sb.append(entityId).append("]\n");

        boolean hasNeeds = false;
        for (String publicKeyHash : permissions.rolesWrite) {

            if (hasNeeds == false) {
                sb.append(" > needs: ");
            } else {
                sb.append(" >        ");
            }

            MessagePublicKeyDto key = d.io.publicKeyOrNull(partitionKey, publicKeyHash);
            if (key != null && key.getAlias() != null) {
                sb.append(key.getAlias()).append(" - ").append(publicKeyHash).append("]");
            } else {
                sb.append(publicKeyHash);
            }

            sb.append("\n");
            hasNeeds = true;
        }
        if (hasNeeds == false) {
            sb.append(" > needs: [no write roles exist!]\n");
        }

        boolean hasOwns = false;
        Set<MessagePrivateKeyDto> rights = this.d.currentRights.getRightsWrite();
        for (MessagePrivateKeyDto privateKey : rights) {
            if (hasOwns == false) {
                sb.append(" > roles: ");
            } else {
                sb.append(" >        ");
            }
            sb.append(d.encryptor.getAlias(partitionKey, privateKey)).append(" - ").append(d.encryptor.getPublicKeyHash(privateKey));
            if (this.d.securityCastleManager.getSignKey(d.encryptor.getPublicKeyHash(privateKey)) == null) {
                sb.append(" [lookup failed!!]");
            }
            sb.append("\n");
            hasOwns = true;
        }

        if (hasOwns == false) {
            sb.append(" > roles: [no access rights]\n");
        }

        // Throw an exception which we will write to the stack
        try {
            return new WebApplicationException(sb.toString(), Response.Status.UNAUTHORIZED);
        } catch (Throwable ex) {
            this.LOG.warn(ex);
            return new WebApplicationException(sb.toString(), Response.Status.UNAUTHORIZED);
        }
    }

    public RuntimeException buildReadException(IPartitionKey partitionKey, @Nullable UUID castleId, @DaoId UUID objId, EffectivePermissions permissions, boolean showStack)
    {
        StringBuilder sb = new StringBuilder();
        sb.append("Access denied while attempting to read object [");
        DataContainer container = d.io.getRawOrNull(PUUID.from(partitionKey, objId));
        if (container != null) {
            sb.append(container.getPayloadClazz()).append(":");
        }
        sb.append(objId).append("]\n");

        sb.append(" > castle: ");
        if (castleId != null) {
            sb.append(castleId).append("\n");
        } else {
            sb.append("[missing!!]\n");
        }

        boolean hasNeeds = false;
        for (String publicKeyHash : permissions.rolesRead) {

            if (hasNeeds == false) {
                sb.append(" > needs: ");
            } else {
                sb.append(" >        ");
            }

            MessagePublicKeyDto roleKey = d.io.publicKeyOrNull(partitionKey, publicKeyHash);
            @Hash String roleKeyAlias = roleKey != null ? d.encryptor.getAlias(partitionKey, roleKey) : publicKeyHash;
            sb.append(roleKeyAlias).append(" - ").append(publicKeyHash).append("]");
            if (castleId == null) {
                sb.append(" [castle missing]");
            } else if (this.d.securityCastleManager.hasEncryptKey(partitionKey, castleId, publicKeyHash)) {
                sb.append(" [record found]");
            } else {
                sb.append(" [record missing!!]");
            }
            sb.append("\n");
            hasNeeds = true;
        }
        if (hasNeeds == false) {
            sb.append(" > needs: [no read roles exist!]\n");
        }

        boolean hasOwns = false;
        Set<MessagePrivateKeyDto> rights = this.d.currentRights.getRightsRead();
        for (MessagePrivateKeyDto privateKey : rights) {
            if (hasOwns == false) {
                sb.append(" > roles: ");
            } else {
                sb.append(" >        ");
            }

            String privateKeyPublicHash = d.encryptor.getPublicKeyHash(privateKey);
            sb.append(d.encryptor.getAlias(partitionKey, privateKey)).append(" - ").append(privateKeyPublicHash);
            sb.append("\n");
            hasOwns = true;
        }

        if (hasOwns == false) {
            sb.append(" > roles: [no access rights]\n");
        }

        // Throw an exception which we will write to the stack
        try {
            return new WebApplicationException(sb.toString(), Response.Status.UNAUTHORIZED);
        } catch (Throwable ex) {
            this.LOG.warn(ex);
            return new WebApplicationException(sb.toString(), Response.Status.UNAUTHORIZED);
        }
    }

    public EffectivePermissions perms(BaseDao obj) {
        IPartitionKey partitionKey = obj.partitionKey();
        return new EffectivePermissionBuilder(partitionKey, obj.getId(), obj.getParentId())
                .setUsePostMerged(true)
                .withSuppliedObject(obj)
                .build();
    }

    public EffectivePermissions perms(PUUID id, @Nullable @DaoId UUID parentId, boolean usePostMerged) {
        return new EffectivePermissionBuilder(id.partition(), id.id(), parentId)
                .setUsePostMerged(usePostMerged)
                .build();
    }

    public EffectivePermissions perms(IPartitionKey partitionKey, @DaoId UUID id, @Nullable @DaoId UUID parentId, boolean usePostMerged) {
        return new EffectivePermissionBuilder(partitionKey, id, parentId)
                .setUsePostMerged(usePostMerged)
                .build();
    }

    public void authorizeEntity(IRights entity, IRoles to) {
        authorizeEntity(entity, to, true);
    }

    public void authorizeEntity(IRights entity, IRoles to, boolean performMerge) {
        authorizeEntityRead(entity, to, performMerge);
        authorizeEntityWrite(entity, to, performMerge);
    }

    public void authorizeRead(MessagePublicKeyDto publicKey, IRoles to) {
        IPartitionKey partitionKey = d.io.partitionResolver().resolve((BaseDao)to);
        if (d.io.publicKeyOrNull(partitionKey, publicKey.getPublicKeyHash()) == null) {
            d.io.merge(partitionKey, publicKey);
        }
        if (to.getTrustAllowRead().values().contains(publicKey.getPublicKeyHash()) == false) {
            to.getTrustAllowRead().put(publicKey.getAlias(), publicKey.getPublicKeyHash());
            d.io.mergeLater((BaseDao)to);
        }
    }

    public @Nullable MessagePrivateKeyDto getImplicitRightToRead(IRights entity)
    {
        IPartitionKey partitionKey = d.io.partitionResolver().resolve(entity);
        @Alias String alias = entity.getRightsAlias();
        MessagePrivateKeyDto right = entity.getRightsRead().stream()
                .filter(p -> alias.equals(d.encryptor.getAlias(partitionKey, p)))
                .filter(p -> d.encryptor.getPublicKeyHash(p).equals(d.encryptor.getPublicKeyHash(d.encryptor.getTrustOfPublicRead())) == false)
                .findFirst()
                .orElse(null);
        return right;
    }

    public MessagePrivateKeyDto getOrCreateImplicitRightToRead(IRights entity)
    {
        IPartitionKey partitionKey = d.io.partitionResolver().resolve(entity);
        @Alias String alias = entity.getRightsAlias();
        MessagePrivateKeyDto right = entity.getRightsRead().stream()
                .filter(p -> alias.equals(d.encryptor.getAlias(partitionKey, p)))
                .filter(p -> d.encryptor.getPublicKeyHash(p).equals(d.encryptor.getPublicKeyHash(d.encryptor.getTrustOfPublicRead())) == false)
                .findFirst()
                .orElse(null);
        if (right == null) {
            right = new MessagePrivateKeyDto(d.encryptor.genEncryptKeyWithAlias(128, alias));

            entity.getRightsRead().add(right);

            d.io.merge(partitionKey, d.encryptor.getPublicKey(right));
            if (entity instanceof BaseDao) {
                d.io.mergeLater((BaseDao)entity);
            }
        }
        return right;
    }

    public void authorizeEntityRead(IRights entity, IRoles to) {
        authorizeEntityRead(entity, to, true);
    }

    public void authorizeEntityRead(IRights entity, IRoles to, boolean performMerge) {
        MessagePrivateKeyDto right = getOrCreateImplicitRightToRead(entity);
        authorizeEntityRead(right, to, performMerge);

        TokenDto token = d.currentToken.getTokenOrNull();
        if (token != null && entity.getId().equals(token.getUserIdOrNull())) {
            d.eventTokenScopeChanged.fire(new TokenScopeChangedEvent(token));
            d.eventTokenChanged.fire(new TokenStateChangedEvent());
            d.eventNewAccessRights.fire(new NewAccessRightsEvent());
            d.eventRightsValidation.fire(new RightsValidationEvent());
        }

        entity.onAddRight(to);
    }

    public void authorizeEntityRead(MessagePublicKeyDto right, IRoles to) {
        authorizeEntityWrite(right, to, true);
    }

    public void authorizeEntityRead(MessagePublicKeyDto right, IRoles to, boolean performMerge) {
        String hash = d.encryptor.getPublicKeyHash(right);

        // If its not in the chain-of-trust then add it
        BaseDao toObj = (BaseDao)to;
        if (performMerge) {
            IPartitionKey partitionKey = toObj.partitionKey();
            if (d.io.publicKeyOrNull(partitionKey, hash) == null) {
                d.io.merge(partitionKey, new MessagePublicKeyDto(right));
            }
        }

        // Add it to the roles list (if its not already there)
        String alias;
        if (performMerge) {
            IPartitionKey partitionKey = toObj.partitionKey();
            alias = d.encryptor.getAlias(partitionKey, right);
        } else {
            alias = right.getAlias();
        }
        if (to.getTrustAllowRead().containsKey(alias)) {
            String rightHash = to.getTrustAllowRead().get(alias);
            if (hash.equals(rightHash)) {
                return;
            }
        }
        to.getTrustAllowRead().put(alias, hash);

        if (performMerge) {
            d.io.mergeLater(toObj);
        }
    }

    public void authorizeEntityPublicRead(IRoles to) {
        authorizeEntityPublicRead(to, true);
    }

    public void authorizeEntityPublicRead(IRoles to, boolean performMerge) {
        @Hash String hash = d.encryptor.getTrustOfPublicRead().getPublicKeyHash();
        assert hash != null : "@AssumeAssertion(nullness): Must not be null";
        to.getTrustAllowRead().put("public", hash);

        if (performMerge) {
            d.io.mergeLater((BaseDao) to);
        }
    }

    public void authorizeWrite(MessagePublicKeyDto publicKey, IRoles to) {
        IPartitionKey partitionKey = d.io.partitionResolver().resolve((BaseDao)to);
        if (d.io.publicKeyOrNull(partitionKey, publicKey.getPublicKeyHash()) == null) {
            d.io.merge(partitionKey, publicKey);
        }

        if (to.getTrustAllowWrite().values().contains(publicKey.getPublicKeyHash()) == false) {
            to.getTrustAllowWrite().put(publicKey.getAlias(), publicKey.getPublicKeyHash());
            d.io.mergeLater((BaseDao)to);
        }
    }

    public @Nullable MessagePrivateKeyDto getImplicitRightToWrite(IRights entity)
    {
        IPartitionKey partitionKey = d.io.partitionResolver().resolve(entity);
        @Alias String alias = entity.getRightsAlias();
        MessagePrivateKeyDto right = entity.getRightsWrite().stream()
                .filter(p -> alias.equals(d.encryptor.getAlias(partitionKey, p)))
                .filter(p -> d.encryptor.getPublicKeyHash(p).equals(d.encryptor.getPublicKeyHash(d.encryptor.getTrustOfPublicWrite())) == false)
                .findFirst()
                .orElse(null);
        return right;
    }

    public MessagePrivateKeyDto getOrCreateImplicitRightToWrite(IRights entity)
    {
        IPartitionKey partitionKey = d.io.partitionResolver().resolve(entity);
        @Alias String alias = entity.getRightsAlias();
        MessagePrivateKeyDto right = entity.getRightsWrite().stream()
                .filter(p -> alias.equals(d.encryptor.getAlias(partitionKey, p)))
                .filter(p -> d.encryptor.getPublicKeyHash(p).equals(d.encryptor.getPublicKeyHash(d.encryptor.getTrustOfPublicWrite())) == false)
                .findFirst()
                .orElse(null);
        if (right == null) {
            right = new MessagePrivateKeyDto(d.encryptor.genSignKeyWithAlias(alias));

            entity.getRightsWrite().add(right);

            d.io.merge(partitionKey, d.encryptor.getPublicKey(right));
            if (entity instanceof BaseDao) {
                d.io.mergeLater((BaseDao)entity);
            }
        }
        return right;
    }

    public void authorizeEntityWrite(IRights entity, IRoles to) {
        authorizeEntityWrite(entity, to, true);
    }

    public void authorizeEntityWrite(IRights entity, IRoles to, boolean performMerge) {
        MessagePrivateKeyDto right = getOrCreateImplicitRightToWrite(entity);
        authorizeEntityWrite(right, to, performMerge);

        TokenDto token = d.currentToken.getTokenOrNull();
        if (token != null && entity.getId().equals(token.getUserIdOrNull())) {
            d.eventTokenScopeChanged.fire(new TokenScopeChangedEvent(token));
            d.eventNewAccessRights.fire(new NewAccessRightsEvent());
            d.eventTokenChanged.fire(new TokenStateChangedEvent());
            d.eventRightsValidation.fire(new RightsValidationEvent());
        }

        entity.onAddRight(to);
    }

    public void authorizeEntityWrite(MessagePublicKeyDto right, IRoles to) {
        authorizeEntityWrite(right, to, true);
    }

    public void authorizeEntityWrite(MessagePublicKeyDto right, IRoles to, boolean performMerge) {
        String hash = d.encryptor.getPublicKeyHash(right);

        // If its not in the chain-of-trust then add it
        BaseDao toObj = (BaseDao)to;
        if (performMerge == true) {
            IPartitionKey partitionKey = toObj.partitionKey();
            if (d.io.publicKeyOrNull(partitionKey, hash) == null) {
                d.io.merge(partitionKey, new MessagePublicKeyDto(right));
            }
        }

        // Add it to the roles (if it doesnt exist)
        String alias;
        if (performMerge) {
            IPartitionKey partitionKey = toObj.partitionKey();
            alias = d.encryptor.getAlias(partitionKey, right);
        } else {
            alias = right.getAlias();
        }
        if (to.getTrustAllowWrite().containsKey(alias)) {
            String rightHash = to.getTrustAllowWrite().get(alias);
            if (hash.equals(rightHash)) {
                return;
            }
        }
        to.getTrustAllowWrite().put(alias, d.encryptor.getPublicKeyHash(right));

        if (performMerge) {
            d.io.mergeLater(toObj);
        }
    }

    public void authorizeEntityPublicWrite(IRoles to) {
        authorizeEntityPublicWrite(to, true);
    }

    public void authorizeEntityPublicWrite(IRoles to, boolean performMerge) {
        @Hash String hash = d.encryptor.getTrustOfPublicWrite().getPublicKeyHash();
        assert hash != null : "@AssumeAssertion(nullness): Must not be null";
        to.getTrustAllowWrite().put("public", hash);

        if (performMerge) {
            d.io.mergeLater((BaseDao) to);
        }
    }

    public void unauthorizeEntity(IRights entity, IRoles from) {
        unauthorizeEntityRead(entity, from);
        unauthorizeEntityWrite(entity, from);
    }

    public void unauthorizeEntityRead(IRights entity, IRoles from) {

        List<MessagePrivateKeyDto> rights = entity.getRightsRead().stream().collect(Collectors.toList());

        for (MessagePrivateKeyDto right : rights) {
            Map.Entry<String, String> publicKeyHash = from.getTrustAllowRead().entrySet().stream()
                    .filter(p -> p.getValue().equals(d.encryptor.getPublicKeyHash(right)) == true)
                    .findFirst()
                    .orElse(null);
            if (publicKeyHash != null) {
                from.getTrustAllowRead().remove(publicKeyHash.getKey());
                d.io.mergeLater((BaseDao) from);
            }
        }

        entity.onRemoveRight(from);
    }

    public void unauthorizeEntityWrite(IRights entity, IRoles from) {

        List<MessagePrivateKeyDto> rights = entity.getRightsWrite().stream().collect(Collectors.toList());

        for (MessagePrivateKeyDto right : rights) {
            Map.Entry<String, String> publicKeyHash = from.getTrustAllowWrite().entrySet().stream()
                    .filter(p -> p.getValue().equals(d.encryptor.getPublicKeyHash(right)) == true)
                    .findFirst()
                    .orElse(null);
            if (publicKeyHash != null) {
                from.getTrustAllowWrite().remove(publicKeyHash.getKey());
                d.io.mergeLater((BaseDao) from);
            }
        }
    }

    public void unauthorizeAlias(IRoles roles, @Alias String alias) {
        unauthorizeAliasRead(roles, alias);
        unauthorizeAliasWrite(roles, alias);
    }

    public void unauthorizeAliasRead(IRoles roles, @Alias String alias) {
        roles.getTrustAllowRead().remove(alias);
        d.io.mergeLater((BaseDao) roles);
    }

    public void unauthorizeAliasWrite(IRoles roles, @Alias String alias) {
        roles.getTrustAllowWrite().remove(alias);
        d.io.mergeLater((BaseDao) roles);
    }

    public void unauthorizeAlias(IRights rights, @Alias String alias) {
        unauthorizeAliasRead(rights, alias);
        unauthorizeAliasWrite(rights, alias);
    }

    public void unauthorizeAliasRead(IRights rights, @Alias String alias) {

        IPartitionKey partitionKey = d.io.partitionResolver().resolve(rights);
        List<MessagePrivateKeyDto> rs = rights.getRightsRead()
                .stream()
                .filter(p -> alias.equals(d.encryptor.getPublicKeyHash(p)) == true ||
                        alias.equals(d.encryptor.getAlias(partitionKey, p)) == true)
                .collect(Collectors.toList());
        for (MessagePrivateKeyDto r : rs) {
            rights.getRightsRead().remove(r);
            d.io.mergeLater((BaseDao)rights);
        }
    }

    public void unauthorizeAliasWrite(IRights rights, @Alias String alias) {

        IPartitionKey partitionKey = d.io.partitionResolver().resolve(rights);
        List<MessagePrivateKeyDto> rs = rights.getRightsWrite()
                .stream()
                .filter(p -> alias.equals(d.encryptor.getPublicKeyHash(p)) == true ||
                        alias.equals(d.encryptor.getAlias(partitionKey, p)) == true)
                .collect(Collectors.toList());
        for (MessagePrivateKeyDto r : rs) {
            rights.getRightsWrite().remove(r);
            d.io.mergeLater((BaseDao)rights);
        }
    }
}
