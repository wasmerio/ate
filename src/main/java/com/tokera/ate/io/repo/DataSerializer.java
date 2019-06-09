package com.tokera.ate.io.repo;

import com.google.common.cache.Cache;
import com.google.common.cache.CacheBuilder;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.api.ISecurityCastleFactory;
import com.tokera.ate.providers.PartitionKeySerializer;
import com.tokera.ate.scopes.Startup;
import com.tokera.ate.common.Immutalizable;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.dao.IRights;
import com.tokera.ate.dao.IRoles;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.security.EffectivePermissionBuilder;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.EffectivePermissions;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.security.SecurityCastleContext;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import java.lang.reflect.Field;
import java.util.*;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;

@Startup
@ApplicationScoped
public class DataSerializer {

    private AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;

    public DataSerializer() {
    }

    @PostConstruct
    public void init() {
        this.LOG.setLogClazz(DataSerializer.class);
    }

    private static Cache<String, BaseDao> decryptCacheObj = CacheBuilder.newBuilder()
            .maximumSize(1000)
            .expireAfterWrite(10, TimeUnit.MINUTES)
            .build();

    private static Cache<String, byte[]> decryptCacheData = CacheBuilder.newBuilder()
            .maximumSize(10000)
            .expireAfterWrite(10, TimeUnit.MINUTES)
            .build();

    private void writeRightPublicKeysForDataObject(BaseDao obj, DataPartition kt) {
        DataPartitionChain chain = kt.getChain();

        // If the entity has rights then make sure they are held within the chain
        // and if they are not then generate messages that will insert them
        if (obj instanceof IRights) {
            IRights rights = (IRights) obj;

            for (MessagePrivateKeyDto key : rights.getRightsRead()) {
                if (chain.hasPublicKey(key.getPublicKeyHash()) == false) {
                    MessagePublicKeyDto publicKey = new MessagePublicKeyDto(key);
                    kt.write(publicKey, this.LOG);
                }
            }

            for (MessagePrivateKeyDto key : rights.getRightsWrite()) {
                @Hash String keyHash = key.getPublicKeyHash();
                if (keyHash == null) continue;
                if (chain.hasPublicKey(keyHash) == false) {
                    MessagePublicKeyDto publicKey = new MessagePublicKeyDto(key);
                    kt.write(publicKey, this.LOG);
                }
            }
        }
    }

    private void writeRolePublicKeysForDataObject(BaseDao obj, DataPartition kt) {
        DataPartitionChain chain = kt.getChain();

        // If we are crossing from our request partition then we need to scan for
        // other public toPutKeys and import them into this partition
        if (obj instanceof IRoles) {
            IRoles roles = (IRoles)obj;

            for (String publicKeyHash : roles.getTrustAllowRead().values()) {
                if (chain.hasPublicKey(publicKeyHash)) continue;
                MessagePublicKeyDto publicKey = d.io.publicKeyOrNull(kt.partitionKey(), publicKeyHash);
                if (publicKey == null) continue;
                kt.write(publicKey, this.LOG);
            }

            for (String publicKeyHash : roles.getTrustAllowWrite().values()) {
                if (chain.hasPublicKey(publicKeyHash)) continue;
                MessagePublicKeyDto publicKey = d.io.publicKeyOrNull(kt.partitionKey(), publicKeyHash);
                if (publicKey == null) continue;
                kt.write(publicKey, this.LOG);
            }
        }
    }

    private void writePermissionPublicKeysForDataObject(EffectivePermissions permissions, DataPartition kt) {
        DataPartitionChain chain = kt.getChain();

        // Write all the public toPutKeys that the chain is unaway of
        for (String publicKeyHash : permissions.rolesWrite)
        {
            // Add the public side of this key if its not already added
            if (chain.hasPublicKey(publicKeyHash) == false)
            {
                // We can only sign if we have a private key for the pair
                MessagePrivateKeyDto privateKey = this.d.securityCastleManager.getSignKey(publicKeyHash);
                if (privateKey == null) continue;

                // Add the key
                Iterable<MessageKeyPartDto> keyParts = privateKey.getPublicParts();
                @Hash String keyHash = privateKey.getPublicKeyHash();
                if (keyParts == null) continue;
                if (keyHash == null) continue;
                MessagePublicKeyDto publicKey = new MessagePublicKeyDto(keyParts, keyHash);

                String alias = privateKey.getAlias();
                if (alias != null) {
                    publicKey.setAlias(alias);
                }

                kt.write(publicKey, this.LOG);
            }
        }
    }

    private void writePublicKeysForDataObject(BaseDao obj, DataPartition kt) {
        writeRightPublicKeysForDataObject(obj, kt);
        writeRolePublicKeysForDataObject(obj, kt);
    }

    private void updateHeaderWithRolesForDataObject(BaseDao obj, MessageDataHeaderDto header)
    {
        // Make sure all the object rights are properly added
        boolean inheritRead = true;
        boolean inheritWrite = true;
        Set<String> allowRead = new HashSet<>();
        Set<String> allowWrite = new HashSet<>();
        if (obj instanceof IRoles) {
            IRoles roles = (IRoles)obj;

            inheritRead = roles.getTrustInheritRead();
            inheritWrite = roles.getTrustInheritWrite();

            allowRead.addAll(roles.getTrustAllowRead().values());
            allowWrite.addAll(roles.getTrustAllowWrite().values());
        }

        // Set the header
        header.setInheritRead(inheritRead);
        header.setInheritWrite(inheritWrite);
        header.setAllowRead(allowRead);
        header.setAllowWrite(allowWrite);
    }

    private void updateHeaderWithImplicitAuthority(BaseDao obj, MessageDataHeaderDto header)
    {
        Set<String> implicitAuthority = new HashSet<>();

        Field implicitField = MapTools.getOrNull(d.daoParents.getAllowedDynamicImplicitAuthority(), obj.getClass());
        if (implicitField != null) {
            try {
                Object implicitDomainObj = implicitField.get(obj);
                if (implicitDomainObj != null) {
                    implicitAuthority.add(implicitDomainObj.toString());
                }
            } catch (IllegalAccessException e) {
                d.genericLogger.warn(e);
            }
        }

        header.setImplicitAuthority(implicitAuthority);
    }

    private MessageDataHeaderDto buildHeaderForDataObject(BaseDao obj, UUID castleId)
    {
        UUID version = obj.version;
        if (version == null) {
            version = UUID.randomUUID();
            obj.version = version;
        }

        MessageDataHeaderDto header = new MessageDataHeaderDto(
                obj.getId(),
                castleId,
                version,
                obj.previousVersion,
                obj.getClass());

        updateHeaderWithRolesForDataObject(obj, header);
        updateHeaderWithImplicitAuthority(obj, header);

        @DaoId UUID parentId = obj.getParentId();
        if (parentId != null) {
            header.setParentId(parentId);
        }

        Set<UUID> previousVersions = obj.mergesVersions;
        if (previousVersions != null) {
            header.getMerges().copyFrom(previousVersions);
        }

        return header;
    }
    
    public MessageBaseDto toDataMessage(BaseDao obj, DataPartition kt, boolean isDeleted, boolean allowSavingOfChildren)
    {
        // Build a header for a new version of the data object
        IPartitionKey partitionKey = kt.partitionKey();
        BaseDao.newVersion(obj);

        // Get the partition and declare a list of message that we will write to Kafka
        writePublicKeysForDataObject(obj, kt);

        // Get the effective permissions for a object
        EffectivePermissions permissions = new EffectivePermissionBuilder(partitionKey, obj.getId(), obj.getParentId())
                .setUsePostMerged(true)
                .build();

        // Generate an encryption key for this data object
        SecurityCastleContext castle = d.securityCastleManager.makeCastle(partitionKey, permissions);
        MessageDataHeaderDto header = buildHeaderForDataObject(obj, castle.id);
        
        // Embed the payload if one exists
        byte[] byteStream = null;
        byte[] encPayload = null;
        if (isDeleted == false)
        {
            // Encrypt the payload and add it to the data message
            byteStream = d.os.serializeObj(obj);
            encPayload = d.encryptor.encryptAes(castle.key, byteStream);
        }

        // Sign the data message
        MessageDataDigestDto digest = d.dataSignatureBuilder.signDataMessage(header, encPayload, permissions);

        // Cache it for faster decryption
        if (byteStream != null && digest != null) {
            @Hash String cacheHash = d.encryptor.hashMd5AndEncode(castle.key, digest.getDigestBytes());
            this.decryptCacheData.put(cacheHash, byteStream);
        }

        // Write all the public toPutKeys that the chain is unaway of
        writePermissionPublicKeysForDataObject(permissions, kt);
        
        // Make sure we are actually writing something to Kafka
        if (digest == null) {
            throw d.authorization.buildWriteException(partitionKey, obj.getId(), permissions, false);
        }

        // Create the message skeleton
        return new MessageDataDto(header, digest, encPayload);
    }

    public @Nullable BaseDao fromDataMessage(IPartitionKey partitionKey, @Nullable MessageDataMetaDto msg, boolean shouldThrow)
    {
        if (msg == null) return null;
        return fromDataMessage(partitionKey, msg.getData(), shouldThrow);
    }
    
    protected @Nullable BaseDao fromDataMessage(IPartitionKey partitionKey, @Nullable MessageDataDto msg, boolean shouldThrow)
    {
        if (msg == null || msg.hasPayload() == false) {
            return null;
        }

        BaseDao ret = readObjectFromDataMessage(partitionKey, msg, shouldThrow);
        if (ret == null) return null;

        validateObjectAfterRead(ret, msg);
        return ret;
    }

    @SuppressWarnings({"unchecked"})
    private <T extends BaseDao> @Nullable T lintDataObject(@Nullable T _orig, MessageDataDto msg) {
        T orig = _orig;
        if (orig == null) return null;

        MessageDataHeaderDto header = msg.getHeader();

        Object cloned = d.merger.cloneObject(orig);
        if (cloned == null) return null;
        T ret = (T)cloned;
        ret.version = header.getVersionOrThrow();
        ret.previousVersion = header.getPreviousVersion();
        ret.mergesVersions = header.getMerges();

        Field implicitAuthorityField = d.daoParents.getAllowedDynamicImplicitAuthoritySimple().getOrDefault(header.getPayloadClazzOrThrow(), null);
        if (implicitAuthorityField != null) {
            try {
                implicitAuthorityField.set(ret, header.getImplicitAuthority().stream().findFirst().orElse(null));
            } catch (IllegalAccessException e) {
                throw new RuntimeException(e);
            }
        }

        return ret;
    }

    @SuppressWarnings({"unchecked"})
    protected @Nullable BaseDao readObjectFromDataMessage(IPartitionKey partitionKey, MessageDataDto msg, boolean shouldThrow)
    {
        // We need to decrypt the data using an encryption key, search for it
        // using all the private toPutKeys we have in our token
        byte[] aesKey = getAesKeyForHeader(partitionKey, msg.getHeader(), shouldThrow);
        if (aesKey == null) return null;

        // Compute what bytes to use for the hash function
        byte[] hashBytes;
        MessageDataDigestDto digest = msg.getDigest();
        if (digest != null) {
            hashBytes = digest.getDigestBytes();
        } else {
            hashBytes = msg.getPayloadBytes();
        }
        if (hashBytes == null) return null;

        // Create a hash from the aesKey and encrypt payload bytes
        @Hash String cacheKey = d.encryptor.hashMd5AndEncode(aesKey, hashBytes);
        try {
            BaseDao orig = this.decryptCacheObj.get(cacheKey, () -> {
                BaseDao ret = readObjectFromDataMessageInternal(cacheKey, aesKey, msg);
                if (ret == null) throw new RuntimeException("Failed to deserialize the data object.");
                if (ret instanceof Immutalizable) ((Immutalizable)ret).immutalize();
                return ret;
            });
            return lintDataObject(orig, msg);
        } catch (ExecutionException e) {
            BaseDao orig = readObjectFromDataMessageInternal(cacheKey, aesKey, msg);
            return lintDataObject(orig, msg);
        }
    }

    private byte @Nullable [] readDataFromDataMessageInternal(byte[] aesKey, MessageDataDto msg)
    {
        byte[] encPayloadBytes = msg.getPayloadBytes();
        if (encPayloadBytes == null) return null;
        return d.encryptor.decryptAes(aesKey, encPayloadBytes);
    }

    @SuppressWarnings({"unchecked"})
    private @Nullable BaseDao readObjectFromDataMessageInternal(@Hash String cacheKey, byte[] aesKey, MessageDataDto msg)
    {
        byte[] payloadBytes;
        try {
            payloadBytes = this.decryptCacheData.get(cacheKey, () -> {
                byte[] data = readDataFromDataMessageInternal(aesKey, msg);
                if (data == null) throw new RuntimeException("Failed to recode the bytes from the stream.");
                return data;
            });
        } catch (ExecutionException e) {
            payloadBytes = readDataFromDataMessageInternal(aesKey, msg);
        }
        if (payloadBytes == null) return null;

        // Find the type of object this is
        String clazzName = msg.getHeader().getPayloadClazzOrThrow();
        Class<BaseDao> clazz = d.serializableObjectsExtension.findClass(clazzName, BaseDao.class);

        // Decrypt the data entity back into its original form and return it
        return d.os.deserializeObj(payloadBytes, clazz);
    }

    private void validateObjectAfterRead(BaseDao ret, MessageDataDto msg)
    {
        // The ID must match the header
        MessageDataHeaderDto header = msg.getHeader();
        @DaoId UUID id = header.getIdOrThrow();
        if (id.equals(ret.getId()) == false) {
            throw new RuntimeException("Read access denied (id does not match) - ID=" + id);
        }

        // Make sure the deserialized type matches the header
        if (header.getPayloadClazzOrThrow().equals(ret.getClass().getName()) == false) {
            throw new RuntimeException("Read access denied (payload types do not match) - ID=" + id);
        }
    }

    private byte @Nullable [] getAesKeyForHeader(IPartitionKey partitionKey, MessageDataHeaderDto header, boolean shouldThrow)
    {
        SecurityCastleContext castle = d.securityCastleManager
                .enterCastle(partitionKey,
                             header.getCastleId(),
                             this.d.currentRights.getRightsRead());
        if (castle == null) {
            if (shouldThrow == true) {
                EffectivePermissions permissions = d.authorization.perms(partitionKey, header.getIdOrThrow(), header.getParentId(), false);
                throw d.authorization.buildReadException(partitionKey, header.getCastleIdOrThrow(), header.getIdOrThrow(), permissions, true);
            }
            return null;
        }
        return castle.key;
    }
}
