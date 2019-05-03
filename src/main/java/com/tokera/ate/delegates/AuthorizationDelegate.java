package com.tokera.ate.delegates;

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
import java.io.PrintWriter;
import java.io.StringWriter;
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
    private AteDelegate d = AteDelegate.getUnsafe();

    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;

    private int defaultKeySize = 128;

    public boolean canRead(@Nullable BaseDao obj)
    {
        if (obj == null) return false;
        return canRead(obj.getId(), obj.getParentId());
    }

    public boolean canRead(@Nullable @DaoId UUID _id, @Nullable @DaoId UUID parentId)
    {
        @DaoId UUID id = _id;
        if (id == null) return false;

        // If its in the cache then we can obviously read it
        if (d.memoryCacheIO.exists(id) == true) return true;

        // Otherwise we need to compute some permissions for it
        EffectivePermissions perms = d.authorization.perms(id, parentId, true);
        return perms.canRead(d.currentRights);
    }

    public boolean canWrite(@Nullable BaseDao obj)
    {
        if (obj == null) return false;
        return canWrite(obj.getId(), obj.getParentId());
    }

    public void ensureCanWrite(BaseDao obj)
    {
        if (canWrite(obj) == false) {
            EffectivePermissions permissions = d.authorization.perms(obj);
            throw buildWriteException(obj.getId(), permissions, true);
        }
    }

    public boolean canWrite(@Nullable @DaoId UUID _id, @Nullable @DaoId UUID parentId)
    {
        @DaoId UUID id = _id;
        if (id == null) return false;
        EffectivePermissions perms = d.authorization.perms(id, parentId, true);
        return perms.canWrite(d.currentRights);
    }

    public RuntimeException buildWriteException(@DaoId UUID entityId, EffectivePermissions permissions, boolean showStack)
    {
        StringBuilder sb = new StringBuilder();
        sb.append("Access denied while attempting to write object [");
        DataContainer container = d.headIO.getRawOrNull(entityId);
        if (container != null) {
            sb.append(container.getPayloadClazz()).append(":");
        }
        sb.append(entityId).append("]\n");

        boolean hasNeeds = false;
        for (String publicKeyHash : permissions.rolesWrite) {

            if (hasNeeds == false) {
                sb.append(" > needs: ");
            } else {
                sb.append(" >        ");
            }

            MessagePublicKeyDto key = d.headIO.publicKeyOrNull(publicKeyHash);
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
            sb.append(d.encryptor.getAlias(privateKey)).append(" - ").append(d.encryptor.getPublicKeyHash(privateKey));
            if (this.d.encryptKeyCachePerRequest.getSignKey(d.encryptor.getPublicKeyHash(privateKey)) == null) {
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

    public RuntimeException buildReadException(@DaoId UUID objId, EffectivePermissions permissions, boolean showStack)
    {
        StringBuilder sb = new StringBuilder();
        sb.append("Access denied while attempting to read object [");
        DataContainer container = d.headIO.getRawOrNull(objId);
        if (container != null) {
            sb.append(container.getPayloadClazz()).append(":");
        }
        sb.append(objId).append("]\n");

        @Secret String encKeyHash = permissions.encryptKeyHash;
        sb.append(" > encKey: ");
        if (encKeyHash != null) {
            MessagePublicKeyDto key = d.headIO.publicKeyOrNull(encKeyHash);
            sb.append(key != null ? d.encryptor.getPublicKeyHash(key) : encKeyHash);
            sb.append("\n");

            boolean hasNeeds = false;
            for (String publicKeyHash : permissions.rolesRead) {

                if (hasNeeds == false) {
                    sb.append(" > needs: ");
                } else {
                    sb.append(" >        ");
                }

                MessagePublicKeyDto roleKey = d.headIO.publicKeyOrNull(publicKeyHash);
                @Hash String roleKeyAlias = roleKey != null ? d.encryptor.getAlias(roleKey) : publicKeyHash;
                sb.append(roleKeyAlias).append(" - ").append(publicKeyHash).append("]");
                if (this.d.encryptKeyCachePerRequest.hasEncryptKey(encKeyHash, publicKeyHash)) {
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
                sb.append(d.encryptor.getAlias(privateKey)).append(" - ").append(privateKeyPublicHash);
                sb.append("\n");
                hasOwns = true;
            }

            if (hasOwns == false) {
                sb.append(" > roles: [no access rights]\n");
            }
        } else {
            sb.append("[missing!!]");
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
        return perms(obj.getId(), obj.getParentId(), true);
    }

    public EffectivePermissions perms(@DaoId UUID id, @Nullable @DaoId UUID parentId, boolean usePostMerged) {
        return new EffectivePermissionBuilder(d.headIO, id, parentId)
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

    public void authorizeRead(@Alias String alias, @Hash String keyHash, IRoles to) {
        if (to.getTrustAllowRead().values().contains(keyHash) == false) {
            to.getTrustAllowRead().put(alias, keyHash);
            d.headIO.mergeLater((BaseDao)to);
        }
    }

    public @Nullable MessagePrivateKeyDto getImplicitRightToRead(IRights entity)
    {
        @Alias String alias = entity.getRightsAlias();
        MessagePrivateKeyDto right = entity.getRightsRead().stream()
                .filter(p -> alias.equals(d.encryptor.getAlias(p)))
                .filter(p -> d.encryptor.getPublicKeyHash(p).equals(d.encryptor.getPublicKeyHash(d.encryptor.getTrustOfPublicRead())) == false)
                .findFirst()
                .orElse(null);
        return right;
    }

    public MessagePrivateKeyDto getOrCreateImplicitRightToRead(IRights entity)
    {
        @Alias String alias = entity.getRightsAlias();
        MessagePrivateKeyDto right = entity.getRightsRead().stream()
                .filter(p -> alias.equals(d.encryptor.getAlias(p)))
                .filter(p -> d.encryptor.getPublicKeyHash(p).equals(d.encryptor.getPublicKeyHash(d.encryptor.getTrustOfPublicRead())) == false)
                .findFirst()
                .orElse(null);
        if (right == null) {
            right = new MessagePrivateKeyDto(d.encryptor.genEncryptKeyNtru(128, alias));

            entity.getRightsRead().add(right);

            d.headIO.merge(d.encryptor.getPublicKey(right));
            if (entity instanceof BaseDao) {
                d.headIO.mergeLater((BaseDao)entity);
            }
        }
        return right;
    }

    public void authorizeEntityRead(IRights entity, IRoles to) {
        authorizeEntityRead(entity, to, true);
    }

    public void authorizeEntityRead(IRights entity, IRoles to, boolean performMerge) {
        MessagePrivateKeyDto right = getOrCreateImplicitRightToRead(entity);

        String alias = d.encryptor.getAlias(right);
        if (to.getTrustAllowRead().containsKey(alias)) {
            String rightHash = to.getTrustAllowRead().get(alias);
            if (d.encryptor.getPublicKeyHash(right).equals(rightHash)) {
                return;
            }
        }

        to.getTrustAllowRead().put(alias, d.encryptor.getPublicKeyHash(right));

        // The encryption toPutKeys need to be rebuilt as otherwise the permissions
        // will not really take effect if one has access to the history of the
        // distributed commit log
        d.daoHelper.generateEncryptKey(to);

        if (performMerge) {
            d.headIO.mergeLater((BaseDao) to);
        }

        TokenDto token = d.currentToken.getTokenOrNull();
        if (token != null && entity.getId().equals(token.getUserId())) {
            d.eventTokenScopeChanged.fire(new TokenScopeChangedEvent(token));
            d.eventNewAccessRights.fire(new NewAccessRightsEvent());
            d.eventTokenChanged.fire(new TokenStateChangedEvent());
        }

        entity.onAddRight(to);
    }

    public void authorizeWrite(@Alias String alias, @Hash String keyHash, IRoles to) {
        if (to.getTrustAllowWrite().values().contains(keyHash) == false) {
            to.getTrustAllowWrite().put(alias, keyHash);
            d.headIO.mergeLater((BaseDao)to);
        }
    }

    public @Nullable MessagePrivateKeyDto getImplicitRightToWrite(IRights entity)
    {
        @Alias String alias = entity.getRightsAlias();
        MessagePrivateKeyDto right = entity.getRightsWrite().stream()
                .filter(p -> alias.equals(d.encryptor.getAlias(p)))
                .filter(p -> d.encryptor.getPublicKeyHash(p).equals(d.encryptor.getPublicKeyHash(d.encryptor.getTrustOfPublicWrite())) == false)
                .findFirst()
                .orElse(null);
        return right;
    }

    public MessagePrivateKeyDto getOrCreateImplicitRightToWrite(IRights entity)
    {
        @Alias String alias = entity.getRightsAlias();
        MessagePrivateKeyDto right = entity.getRightsWrite().stream()
                .filter(p -> alias.equals(d.encryptor.getAlias(p)))
                .filter(p -> d.encryptor.getPublicKeyHash(p).equals(d.encryptor.getPublicKeyHash(d.encryptor.getTrustOfPublicWrite())) == false)
                .findFirst()
                .orElse(null);
        if (right == null) {
            right = new MessagePrivateKeyDto(d.encryptor.genSignKeyNtru(defaultKeySize, alias));

            entity.getRightsWrite().add(right);

            d.headIO.merge(d.encryptor.getPublicKey(right));
            if (entity instanceof BaseDao) {
                d.headIO.mergeLater((BaseDao)entity);
            }
        }
        return right;
    }

    public void authorizeEntityWrite(IRights entity, IRoles to) {
        authorizeEntityWrite(entity, to, true);
    }

    public void authorizeEntityWrite(IRights entity, IRoles to, boolean performMerge) {

        MessagePrivateKeyDto right = getOrCreateImplicitRightToWrite(entity);

        String alias = d.encryptor.getAlias(right);
        if (to.getTrustAllowWrite().containsKey(alias)) {
            String rightHash = to.getTrustAllowWrite().get(alias);
            if (d.encryptor.getPublicKeyHash(right).equals(rightHash)) {
                return;
            }
        }

        to.getTrustAllowWrite().put(d.encryptor.getAlias(right), d.encryptor.getPublicKeyHash(right));

        if (performMerge) {
            d.headIO.mergeLater((BaseDao) to);
        }

        TokenDto token = d.currentToken.getTokenOrNull();
        if (token != null && entity.getId().equals(token.getUserId())) {
            d.eventTokenScopeChanged.fire(new TokenScopeChangedEvent(token));
            d.eventNewAccessRights.fire(new NewAccessRightsEvent());
            d.eventTokenChanged.fire(new TokenStateChangedEvent());
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
                d.headIO.mergeLater((BaseDao) from);
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
                d.headIO.mergeLater((BaseDao) from);
            }
        }
    }

    public void unauthorizeAlias(IRoles roles, @Alias String alias) {
        unauthorizeAliasRead(roles, alias);
        unauthorizeAliasWrite(roles, alias);
    }

    public void unauthorizeAliasRead(IRoles roles, @Alias String alias) {
        roles.getTrustAllowRead().remove(alias);
        d.headIO.mergeLater((BaseDao) roles);
    }

    public void unauthorizeAliasWrite(IRoles roles, @Alias String alias) {
        roles.getTrustAllowWrite().remove(alias);
        d.headIO.mergeLater((BaseDao) roles);
    }

    public void unauthorizeAlias(IRights rights, @Alias String alias) {
        unauthorizeAliasRead(rights, alias);
        unauthorizeAliasWrite(rights, alias);
    }

    public void unauthorizeAliasRead(IRights rights, @Alias String alias) {

        List<MessagePrivateKeyDto> rs = rights.getRightsRead()
                .stream()
                .filter(p -> alias.equals(d.encryptor.getPublicKeyHash(p)) == true ||
                        alias.equals(d.encryptor.getAlias(p)) == true)
                .collect(Collectors.toList());
        for (MessagePrivateKeyDto r : rs) {
            rights.getRightsRead().remove(r);
            d.headIO.mergeLater((BaseDao)rights);
        }
    }

    public void unauthorizeAliasWrite(IRights rights, @Alias String alias) {

        List<MessagePrivateKeyDto> rs = rights.getRightsWrite()
                .stream()
                .filter(p -> alias.equals(d.encryptor.getPublicKeyHash(p)) == true ||
                        alias.equals(d.encryptor.getAlias(p)) == true)
                .collect(Collectors.toList());
        for (MessagePrivateKeyDto r : rs) {
            rights.getRightsWrite().remove(r);
            d.headIO.mergeLater((BaseDao)rights);
        }
    }

    public int getDefaultKeySize() {
        return defaultKeySize;
    }

    public void setDefaultKeySize(int defaultKeySize) {
        this.defaultKeySize = defaultKeySize;
    }
}
