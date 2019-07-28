package com.tokera.ate.delegates;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.dao.IRights;
import com.tokera.ate.dao.IRoles;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.base.BaseDaoInternal;
import com.tokera.ate.dao.enumerations.PermissionPhase;
import com.tokera.ate.dto.EffectivePermissions;
import com.tokera.ate.dto.RolesPairDto;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.events.NewAccessRightsEvent;
import com.tokera.ate.events.RightsValidationEvent;
import com.tokera.ate.events.TokenScopeChangedEvent;
import com.tokera.ate.events.TokenStateChangedEvent;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.DataContainer;
import com.tokera.ate.providers.PartitionKeySerializer;
import com.tokera.ate.scopes.Startup;
import com.tokera.ate.security.EffectivePermissionBuilder;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;
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

    public boolean canRead(@Nullable PUUID _id)
    {
        PUUID id = _id;
        if (id == null) return false;
        return canRead(id.partition(), id.id());
    }

    public boolean canRead(IPartitionKey partitionKey, @DaoId UUID id)
    {
        // If its in the cache then we can obviously read it
        if (d.memoryRequestCacheIO.exists(PUUID.from(partitionKey, id)) == true) return true;

        // Otherwise we need to compute some permissions for it
        EffectivePermissions perms = d.authorization.perms(null, partitionKey, id, PermissionPhase.BeforeMerge);
        return perms.canRead(d.currentRights);
    }

    public boolean canWrite(@Nullable BaseDao obj)
    {
        if (obj == null) return false;
        EffectivePermissions perms = d.authorization.perms(obj);
        return perms.canWrite(d.currentRights);
    }

    public boolean canWrite(@Nullable PUUID _id)
    {
        PUUID id = _id;
        if (id == null) return false;
        return canWrite(id.partition(), id.id());
    }

    public void ensureCanWrite(BaseDao obj)
    {
        if (canWrite(obj) == false) {
            EffectivePermissions permissions = d.authorization.perms(obj);
            throw buildWriteException(permissions, true);
        }
    }

    public boolean canWrite(IPartitionKey partitionKey, @DaoId UUID id)
    {
        EffectivePermissions perms = d.authorization.perms(null, partitionKey, id, PermissionPhase.BeforeMerge);
        return perms.canWrite(d.currentRights);
    }

    public RuntimeException buildWriteException(EffectivePermissions permissions, boolean showStack)
    {
        return buildWriteException("Access denied while attempting to write object", permissions, showStack);
    }

    public RuntimeException buildWriteException(String msg, EffectivePermissions permissions, boolean showStack)
    {
        IPartitionKey partitionKey = permissions.partitionKey;
        @DaoId UUID entityId = permissions.id;

        StringBuilder sb = new StringBuilder();
        sb.append(msg);
        sb.append(" [");
        DataContainer container = d.io.getRawOrNull(PUUID.from(partitionKey, entityId));
        if (container != null) {
            sb.append(container.getPayloadClazz()).append(":");
        } else {
            BaseDao obj = d.dataStagingManager.find(PUUID.from(partitionKey, entityId));
            if (obj != null) {
                sb.append(BaseDaoInternal.getShortType(obj)).append(":");
            }
        }
        sb.append(entityId).append("]\n");

        if (permissions.type != null) {
            sb.append(" >  type: ").append(permissions.type).append("\n");
        }
        sb.append(" > where: ").append(PartitionKeySerializer.toString(permissions.partitionKey)).append("\n");

        boolean hasNeeds = false;
        for (String publicKeyHash : permissions.rolesWrite) {

            if (hasNeeds == false) {
                sb.append(" > needs: ");
            } else {
                sb.append(" >        ");
            }

            MessagePublicKeyDto key = d.io.publicKeyOrNull(partitionKey, publicKeyHash);
            if (key != null) {
                if (key.getAlias() != null) {
                    sb.append(key.getAlias()).append(" - ").append(publicKeyHash).append("]");
                }
            } else {
                sb.append("[missing] - ").append(publicKeyHash);
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
            if (this.d.dataStagingManager.findPrivateKey(partitionKey, d.encryptor.getPublicKeyHash(privateKey)) == null) {
                sb.append(" [not staged!!]");
            }
            sb.append("\n");
            hasOwns = true;
        }

        if (hasOwns == false) {
            sb.append(" > roles: [no access rights]\n");
        }

        // Throw an exception which we will write to the stack
        String exMsg = sb.toString();
        try {
            return new WebApplicationException(exMsg, Response.Status.UNAUTHORIZED);
        } catch (Throwable ex) {
            this.LOG.warn(ex);
            return new WebApplicationException(exMsg, Response.Status.UNAUTHORIZED);
        }
    }

    public void validateReadOrThrow(PUUID pid) {
        validateReadOrThrow(pid.partition(), pid.id());
    }

    public void validateWriteOrThrow(PUUID pid) {
        validateWriteOrThrow(pid.partition(), pid.id());
    }

    public void validateReadOrThrow(IPartitionKey partitionKey, @DaoId UUID objId) {
        EffectivePermissions permissions = this.perms(null, partitionKey, objId, PermissionPhase.BeforeMerge);
        if (canRead(partitionKey, objId) == false) {
            throw buildReadException(permissions, false);
        }
    }

    public void validateWriteOrThrow(IPartitionKey partitionKey, @DaoId UUID objId) {
        EffectivePermissions permissions = this.perms(null, partitionKey, objId, PermissionPhase.BeforeMerge);
        if (canWrite(partitionKey, objId) == false) {
            throw buildWriteException(permissions, false);
        }
    }

    public RuntimeException buildReadException(EffectivePermissions permissions, boolean showStack)
    {
        return buildReadException("Access denied while attempting to read object", permissions, showStack);
    }

    public RuntimeException buildReadException(String msg, EffectivePermissions permissions, boolean showStack)
    {
        IPartitionKey partitionKey = permissions.partitionKey;
        @DaoId UUID objId = permissions.id;

        StringBuilder sb = new StringBuilder();
        sb.append(msg);
        sb.append(" [");
        DataContainer container = d.io.getRawOrNull(PUUID.from(partitionKey, objId));
        if (container != null) {
            sb.append(container.getPayloadClazz()).append(":");
        }
        sb.append(objId).append("]\n");

        if (permissions.type != null) {
            sb.append(" >  type: ").append(permissions.type).append("\n");
        }
        sb.append(" > where: ").append(PartitionKeySerializer.toString(partitionKey)).append("\n");

        sb.append(" > castle: ");
        UUID castleId = permissions.castleId;
        if (castleId != null) {
            sb.append(castleId);
            if (this.d.securityCastleManager.hasCastle(partitionKey, castleId)) {
                sb.append(" [missing!!]");
            }
            sb.append("\n");
        } else {
            sb.append("[none]\n");
        }

        boolean hasNeeds = false;
        for (String publicKeyHash : permissions.rolesRead) {

            if (hasNeeds == false) {
                sb.append(" > needs: ");
            } else {
                sb.append(" >        ");
            }

            MessagePublicKeyDto roleKey = d.io.publicKeyOrNull(partitionKey, publicKeyHash);
            @Hash String roleKeyAlias = roleKey != null ? d.encryptor.getAlias(partitionKey, roleKey) : "[missing]";
            sb.append(roleKeyAlias).append(" - ").append(publicKeyHash);
            if (castleId == null) {
                sb.append(" [castle unknown]");
            } else if (this.d.securityCastleManager.hasCastle(partitionKey, castleId)) {
                sb.append(" [castle missing]");
            } else if (this.d.securityCastleManager.hasEncryptKey(partitionKey, castleId, publicKeyHash)) {
                sb.append(" [castle key found]");
            } else {
                sb.append(" [castle key missing!!]");
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
            if (castleId == null) {
                sb.append(" [no castle]");
            } else if (this.d.securityCastleManager.hasCastle(partitionKey, castleId)) {
                sb.append(" [castle missing]");
            } else if (this.d.securityCastleManager.hasEncryptKey(partitionKey, castleId, privateKeyPublicHash)) {
                if (permissions.rolesRead.contains(privateKeyPublicHash)) {
                    sb.append(" [record found]");
                } else {
                    sb.append(" [irrelevant record found]");
                }
            } else {
                if (permissions.rolesRead.contains(privateKeyPublicHash)) {
                    sb.append(" [record missing]");
                } else {
                    sb.append(" [irrelevant record missing]");
                }
            }
            sb.append("\n");
            hasOwns = true;
        }

        if (hasOwns == false) {
            sb.append(" > roles: [no access rights]\n");
        }

        // Throw an exception which we will write to the stack
        String exMsg = sb.toString();
        try {
            return new WebApplicationException(exMsg, Response.Status.UNAUTHORIZED);
        } catch (Throwable ex) {
            this.LOG.warn(ex);
            return new WebApplicationException(exMsg, Response.Status.UNAUTHORIZED);
        }
    }

    public EffectivePermissions perms(BaseDao obj) {
        return perms(obj, PermissionPhase.AfterMerge);
    }

    public EffectivePermissions perms(BaseDao obj, PermissionPhase phase) {
        IPartitionKey partitionKey = obj.partitionKey(true);
        return new EffectivePermissionBuilder(BaseDaoInternal.getType(obj), partitionKey, obj.getId())
                .withPhase(phase)
                .withSuppliedObject(obj)
                .build();
    }

    public EffectivePermissions perms(String type, PUUID id) {
        return perms(type, id, PermissionPhase.BeforeMerge);
    }

    public EffectivePermissions perms(String type, PUUID id, PermissionPhase phase) {
        return new EffectivePermissionBuilder(type, id.partition(), id.id())
                .withPhase(phase)
                .build();
    }

    public EffectivePermissions perms(String type, IPartitionKey partitionKey, @DaoId UUID id) {
        return perms(type, partitionKey, id, PermissionPhase.BeforeMerge);
    }

    public EffectivePermissions perms(String type, IPartitionKey partitionKey, @DaoId UUID id, PermissionPhase phase) {
        return new EffectivePermissionBuilder(type, partitionKey, id)
                .withPhase(phase)
                .build();
    }

    public void authorizeEntity(IRights entity, IRoles to) {
        authorizeEntityRead(entity, to);
        authorizeEntityWrite(entity, to);
    }

    public void authorizeRead(MessagePublicKeyDto publicKey, IRoles to) {
        ensureKeyIsThere(publicKey, to);
        if (to.getTrustAllowRead().values().contains(publicKey.getPublicKeyHash()) == false) {
            to.getTrustAllowRead().put(publicKey.getAliasOrHash(), publicKey.getPublicKeyHash());
        }
    }

    public void copy(IRoles from, IRoles to)
    {
        boolean save = false;
        for (Map.Entry<String, String> pair : from.getTrustAllowRead().entrySet()) {
            if (to.getTrustAllowRead().getOrDefault(pair.getKey(), "").equals(pair.getValue()) == false) {
                to.getTrustAllowRead().put(pair.getKey(), pair.getValue());
                save = true;
            }
        }
        for (Map.Entry<String, String> pair : from.getTrustAllowWrite().entrySet()) {
            if (to.getTrustAllowWrite().getOrDefault(pair.getKey(), "").equals(pair.getValue()) == false) {
                to.getTrustAllowWrite().put(pair.getKey(), pair.getValue());
                save = true;
            }
        }
        if (save && to instanceof BaseDao) {
            d.io.mergeLater((BaseDao)to);
        }
    }

    public void copyEffective(BaseDao from, IRoles to)
    {
        EffectivePermissions perms = d.authorization.perms(from);

        boolean save = false;
        for (String role : perms.rolesRead) {
            if (to.getTrustAllowRead().containsValue(role) == false) {
                to.getTrustAllowRead().put(role, role);
                save = true;
            }
        }
        for (String role : perms.rolesWrite) {
            if (to.getTrustAllowWrite().containsValue(role) == false) {
                to.getTrustAllowWrite().put(role, role);
                save = true;
            }
        }
        if (save && to instanceof BaseDao) {
            d.io.mergeLater((BaseDao)to);
        }
    }

    public @Nullable MessagePrivateKeyDto getImplicitRightToRead(IRights entity)
    {
        @Alias String alias = entity.getRightsAlias();
        MessagePrivateKeyDto right = entity.getRightsRead().stream()
                .filter(p -> alias.equals(p.getAliasOrHash()))
                .filter(p -> d.encryptor.getPublicKeyHash(p).equals(d.encryptor.getPublicKeyHash(d.encryptor.getTrustOfPublicRead())) == false)
                .findFirst()
                .orElse(null);
        return right;
    }

    public MessagePrivateKeyDto getOrCreateImplicitRightToRead(IRights entity)
    {
        @Alias String alias = entity.getRightsAlias();
        MessagePrivateKeyDto right = entity.getRightsRead().stream()
                .filter(p -> alias.equals(p.getAliasOrHash()))
                .filter(p -> d.encryptor.getPublicKeyHash(p).equals(d.encryptor.getPublicKeyHash(d.encryptor.getTrustOfPublicRead())) == false)
                .findFirst()
                .orElse(null);
        if (right == null) {
            if (entity.readOnly()) {
                throw new WebApplicationException("Unable to create an implicit right to read for this entity as it is read only.", Response.Status.BAD_REQUEST);
            }
            right = new MessagePrivateKeyDto(d.encryptor.genEncryptKeyWithAlias(128, alias));

            entity.getRightsRead().add(right);
            ensureKeyIsThere(right, entity);
        }
        return right;
    }

    public void authorizeEntityRead(IRights entity, IRoles to) {
        MessagePrivateKeyDto right = getOrCreateImplicitRightToRead(entity);
        ensureKeyIsThere(right, to);

        authorizeEntityRead(right, to);

        TokenDto token = d.currentToken.getTokenOrNull();
        if (token != null && entity.getId().equals(token.getUserIdOrNull())) {
            d.eventTokenScopeChanged.fire(new TokenScopeChangedEvent(token));
            d.eventTokenChanged.fire(new TokenStateChangedEvent());
            d.eventNewAccessRights.fire(new NewAccessRightsEvent());
            d.eventRightsValidation.fire(new RightsValidationEvent());
        }

        entity.onAddRight(to);
    }

    public void authorizeEntity(@Nullable RolesPairDto pair, IRoles to) {
        if (pair == null) return;

        if (pair.read != null) {
            authorizeEntityRead(pair.read, to);
            if (to instanceof BaseDao) {
                d.io.mergeLater(((BaseDao)to));
            }
        }
        if (pair.write != null) {
            authorizeEntityWrite(pair.write, to);
            if (to instanceof BaseDao) {
                d.io.mergeLater(((BaseDao)to));
            }
        }
    }

    public void authorizeEntityRead(MessagePublicKeyDto right, IRoles to) {
        String hash = d.encryptor.getPublicKeyHash(right);

        ensureKeyIsThere(right, to);

        // Add it to the roles list (if its not already there)
        String alias = right.getAliasOrHash();
        if (to.getTrustAllowRead().containsKey(alias)) {
            String rightHash = to.getTrustAllowRead().get(alias);
            if (hash.equals(rightHash)) {
                return;
            }
        }
        to.getTrustAllowRead().put(alias, hash);
    }

    public void authorizeEntityPublicRead(IRoles to) {
        MessagePublicKeyDto key = d.encryptor.getTrustOfPublicRead();
        ensureKeyIsThere(key, to);

        @Hash String hash = key.getPublicKeyHash();
        assert hash != null : "@AssumeAssertion(nullness): Must not be null";
        to.getTrustAllowRead().put("public", hash);
    }

    public void authorizeWrite(MessagePublicKeyDto publicKey, IRoles to) {
        ensureKeyIsThere(publicKey, to);

        if (to.getTrustAllowWrite().values().contains(publicKey.getPublicKeyHash()) == false) {
            to.getTrustAllowWrite().put(publicKey.getAliasOrHash(), publicKey.getPublicKeyHash());
        }
    }

    public @Nullable MessagePrivateKeyDto getImplicitRightToWrite(IRights entity)
    {
        @Alias String alias = entity.getRightsAlias();
        MessagePrivateKeyDto right = entity.getRightsWrite().stream()
                .filter(p -> alias.equals(p.getAliasOrHash()))
                .filter(p -> d.encryptor.getPublicKeyHash(p).equals(d.encryptor.getPublicKeyHash(d.encryptor.getTrustOfPublicWrite())) == false)
                .findFirst()
                .orElse(null);
        return right;
    }

    public MessagePrivateKeyDto getOrCreateImplicitRightToWrite(IRights entity)
    {
        @Alias String alias = entity.getRightsAlias();
        MessagePrivateKeyDto right = entity.getRightsWrite().stream()
                .filter(p -> alias.equals(p.getAliasOrHash()))
                .filter(p -> d.encryptor.getPublicKeyHash(p).equals(d.encryptor.getPublicKeyHash(d.encryptor.getTrustOfPublicWrite())) == false)
                .findFirst()
                .orElse(null);
        if (right == null) {
            if (entity.readOnly()) {
                throw new WebApplicationException("Unable to create an implicit right to write for this entity as it is read only.", Response.Status.BAD_REQUEST);
            }
            right = new MessagePrivateKeyDto(d.encryptor.genSignKeyWithAlias(alias));
            entity.getRightsWrite().add(right);
            ensureKeyIsThere(right, entity);
        }
        return right;
    }

    public void authorizeEntityWrite(IRights entity, IRoles to) {
        MessagePrivateKeyDto right = getOrCreateImplicitRightToWrite(entity);
        authorizeEntityWrite(right, to);
        ensureKeyIsThere(right, to);

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
        String hash = d.encryptor.getPublicKeyHash(right);

        ensureKeyIsThere(right, to);

        // Add it to the roles (if it doesnt exist)
        String alias = right.getAliasOrHash();
        if (to.getTrustAllowWrite().containsKey(alias)) {
            String rightHash = to.getTrustAllowWrite().get(alias);
            if (hash.equals(rightHash)) {
                return;
            }
        }
        to.getTrustAllowWrite().put(alias, d.encryptor.getPublicKeyHash(right));
    }

    public void authorizeEntityPublicWrite(IRoles to) {
        MessagePublicKeyDto key = d.encryptor.getTrustOfPublicWrite();
        ensureKeyIsThere(key, to);

        @Hash String hash = key.getPublicKeyHash();
        assert hash != null : "@AssumeAssertion(nullness): Must not be null";
        to.getTrustAllowWrite().put("public", hash);
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
            }
        }
    }

    public void unauthorizeAlias(IRoles roles, @Alias String alias) {
        unauthorizeAliasRead(roles, alias);
        unauthorizeAliasWrite(roles, alias);
    }

    public void unauthorizeAliasRead(IRoles roles, @Alias String alias) {
        roles.getTrustAllowRead().remove(alias);
    }

    public void unauthorizeAliasWrite(IRoles roles, @Alias String alias) {
        roles.getTrustAllowWrite().remove(alias);
    }

    public void unauthorizeAlias(IRights rights, @Alias String alias) {
        unauthorizeAliasRead(rights, alias);
        unauthorizeAliasWrite(rights, alias);
    }

    public void unauthorizeAliasRead(IRights rights, @Alias String alias) {
        List<MessagePrivateKeyDto> rs = rights.getRightsRead()
                .stream()
                .filter(p -> alias.equals(d.encryptor.getPublicKeyHash(p)) == true ||
                        alias.equals(p.getPublicKeyHash()) == true)
                .collect(Collectors.toList());
        for (MessagePrivateKeyDto r : rs) {
            rights.getRightsRead().remove(r);
        }
    }

    public void unauthorizeAliasWrite(IRights rights, @Alias String alias) {
        List<MessagePrivateKeyDto> rs = rights.getRightsWrite()
                .stream()
                .filter(p -> alias.equals(d.encryptor.getPublicKeyHash(p)) == true ||
                        alias.equals(p.getPublicKeyHash()) == true)
                .collect(Collectors.toList());
        for (MessagePrivateKeyDto r : rs) {
            rights.getRightsWrite().remove(r);
        }
    }

    public void ensureKeyIsThere(MessagePublicKeyDto publicKey, IRoles roles) {
        if (roles instanceof BaseDao) {
            IPartitionKey partitionKey = ((BaseDao)roles).partitionKey(false);
            if (partitionKey != null) {
                ensureKeyIsThere(partitionKey, publicKey);
            }
        }

        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScopeOrNull();
        if (partitionKey != null) {
            ensureKeyIsThere(partitionKey, publicKey);
        }
    }

    public void ensureKeyIsThere(MessagePublicKeyDto publicKey, IRights rights) {
        if (rights instanceof BaseDao) {
            IPartitionKey partitionKey = ((BaseDao)rights).partitionKey(false);
            if (partitionKey != null) {
                ensureKeyIsThere(partitionKey, publicKey);
            }
        } else {
            IPartitionKey partitionKey = d.io.partitionResolver().resolveOrNull(rights);
            if (partitionKey != null) {
                ensureKeyIsThere(partitionKey, publicKey);
            }
        }

        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScopeOrNull();
        if (partitionKey != null) {
            ensureKeyIsThere(partitionKey, publicKey);
        }
    }

    public void ensureKeyIsThere(MessagePublicKeyDto publicKey) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScopeOrNull();
        if (partitionKey != null) {
            ensureKeyIsThere(partitionKey, publicKey);
        }
    }

    public void ensureKeyIsThere(IPartitionKey partitionKey, MessagePublicKeyDto publicKey) {
        if (d.io.publicKeyOrNull(partitionKey, publicKey.getPublicKeyHash()) == null) {
            d.io.merge(partitionKey, publicKey);
        }
    }
}
