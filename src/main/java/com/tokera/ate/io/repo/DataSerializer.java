package com.tokera.ate.io.repo;

import com.google.common.cache.Cache;
import com.google.common.cache.CacheBuilder;
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
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import javax.ws.rs.core.Response;
import java.util.*;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;

@Startup
@ApplicationScoped
public class DataSerializer {

    private AteDelegate d = AteDelegate.getUnsafe();
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

    private byte[] getEncryptKeyForDataObject(BaseDao obj, boolean allowSavingOfChildren) {
        // Get the encryption key we will be using for the data entity
        String encryptKey64 = d.daoHelper.getEncryptKey(obj, false, allowSavingOfChildren);
        if (encryptKey64 == null) {
            StringBuilder sb = new StringBuilder();
            sb.append("No encryption toPutKeys available for this data entity\n");
            for (BaseDao parent : d.daoHelper.getObjAndParents(obj)) {
                sb.append(" - obj [clazz=").append(parent.getClass().getSimpleName()).append(", id=").append(parent.getId());
                if (parent instanceof IRoles) {
                    if (((IRoles)parent).getEncryptKey() != null) {
                        sb.append(", key=yes");
                    } else {
                        sb.append(", key=no");
                    }
                }
                sb.append("]\n");
            }
            throw new RuntimeException(sb.toString());
        }
        return Base64.decodeBase64(encryptKey64);
    }

    private void writeRightPublicKeysForDataObject(BaseDao obj, DataTopic kt) {
        DataTopicChain chain = kt.getChain();

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

    private void writeRolePublicKeysForDataObject(BaseDao obj, DataTopic kt) {
        DataTopicChain chain = kt.getChain();

        // If we are crossing from our requestContext topic then we need to scan for
        // other public toPutKeys and import them into this topic
        if (obj instanceof IRoles) {
            IRoles roles = (IRoles)obj;

            for (String publicKeyHash : roles.getTrustAllowRead().values()) {
                if (chain.hasPublicKey(publicKeyHash)) continue;
                MessagePublicKeyDto publicKey = d.headIO.publicKeyOrNull(publicKeyHash);
                if (publicKey == null) continue;
                kt.write(publicKey, this.LOG);
            }

            for (String publicKeyHash : roles.getTrustAllowWrite().values()) {
                if (chain.hasPublicKey(publicKeyHash)) continue;
                MessagePublicKeyDto publicKey = d.headIO.publicKeyOrNull(publicKeyHash);
                if (publicKey == null) continue;
                kt.write(publicKey, this.LOG);
            }
        }
    }

    private void writePermissionPublicKeysForDataObject(EffectivePermissions permissions, DataTopic kt) {
        DataTopicChain chain = kt.getChain();

        // Write all the public toPutKeys that the chain is unaway of
        for (String publicKeyHash : permissions.rolesWrite)
        {
            // Add the public side of this key if its not already added
            if (chain.hasPublicKey(publicKeyHash) == false)
            {
                // We can only sign if we have a private key for the pair
                MessagePrivateKeyDto privateKey = this.d.encryptKeyCachePerRequest.getSignKey(publicKeyHash);
                if (privateKey == null) continue;

                // Add the key
                byte[] keyBytes = privateKey.getPublicKeyBytes();
                @Hash String keyHash = privateKey.getPublicKeyHash();
                if (keyBytes == null) continue;
                if (keyHash == null) continue;
                MessagePublicKeyDto publicKey = new MessagePublicKeyDto(keyBytes, keyHash);

                String alias = privateKey.getAlias();
                if (alias != null) {
                    publicKey.setAlias(alias);
                }

                kt.write(publicKey, this.LOG);
            }
        }
    }

    private void writePermissionEncryptKeysForDataObject(EffectivePermissions permissions, DataTopic kt, byte[] encryptKey, String encryptKeyHash) {
        DataTopicChain chain = kt.getChain();

        for (String publicKeyHash : permissions.rolesRead)
        {
            // Get the public key
            byte[] publicKeyBytes = chain.getPublicKeyBytes(publicKeyHash);
            if (publicKeyBytes == null) {
                throw new RuntimeException("We encountered a public key that is not yet known to the distributed commit log. Ensure all public toPutKeys are merged before using them in data entities by either calling mergeLater(obj), mergeThreeWay(obj) or mergeThreeWay(publicKeyOrNull).");
            }
            if (publicKeyBytes.length <= 64) {
                throw new RuntimeException("We encountered a public key that does not valid. Ensure all public toPutKeys are merged before using them in data entities by either calling mergeLater(obj), mergeThreeWay(obj) or mergeThreeWay(publicKeyOrNull).");
            }

            // If the key is not available in the kafka topic then we need to add it
            MessageEncryptTextDto encryptText = chain.getEncryptedText(publicKeyHash, encryptKeyHash);
            if (encryptText == null)
            {
                // Encrypt the key
                byte[] encKey = d.encryptor.encryptNtruWithPublic(publicKeyBytes, encryptKey);

                // Create a message and add it
                encryptText = new MessageEncryptTextDto(
                        publicKeyHash,
                        encryptKeyHash,
                        encKey);
                kt.write(encryptText, this.LOG);
            }
        }
    }

    private void writePublicKeysForDataObject(BaseDao obj, DataTopic kt) {
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

    private MessageDataHeaderDto buildHeaderForDataObject(BaseDao obj)
    {
        UUID version = obj.version;
        if (version == null) {
            version = UUID.randomUUID();
            obj.version = version;
        }

        MessageDataHeaderDto header = new MessageDataHeaderDto(
                obj.getId(),
                version,
                obj.previousVersion,
                obj.getClass().getSimpleName());

        updateHeaderWithRolesForDataObject(obj, header);

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
    
    public MessageBaseDto toDataMessage(BaseDao obj, DataTopic kt, boolean isDeleted, boolean allowSavingOfChildren)
    {
        // Build a header for a new version of the data object
        BaseDao.newVersion(obj);
        MessageDataHeaderDto header = buildHeaderForDataObject(obj);

        byte[] encryptKey = getEncryptKeyForDataObject(obj, allowSavingOfChildren);
        String encryptKeyHash = d.encryptor.hashShaAndEncode(encryptKey);
        header.setEncryptKeyHash(encryptKeyHash);

        // Get the topic and declare a list of message that we will write to Kafka
        writePublicKeysForDataObject(obj, kt);

        // Get the effective permissions for a object
        EffectivePermissions permissions = new EffectivePermissionBuilder(d.headIO, obj.getId(), obj.getParentId())
                .setUsePostMerged(true)
                .buildWith(obj);
        
        // Embed the decryption key using all the private toPutKeys that we might have
        writePermissionEncryptKeysForDataObject(permissions, kt, encryptKey, encryptKeyHash);
        
        // Embed the payload if one exists
        byte[] byteStream = null;
        byte[] encPayload = null;
        if (isDeleted == false)
        {
            // Encrypt the payload and add it to the data message
            byteStream = d.os.serializeObj(obj);
            encPayload = d.encryptor.encryptAes(encryptKey, byteStream);
        }

        // Sign the data message
        MessageDataDigestDto digest = d.dataSignatureBuilder.signDataMessage(header, encPayload, permissions);

        // Cache it for faster decryption
        if (byteStream != null && digest != null) {
            @Hash String cacheHash = d.encryptor.hashMd5AndEncode(encryptKey, digest.getDigestBytes());
            this.decryptCacheData.put(cacheHash, byteStream);
        }

        // Write all the public toPutKeys that the chain is unaway of
        writePermissionPublicKeysForDataObject(permissions, kt);
        
        // Make sure we are actually writing something to Kafka
        if (digest == null) {
            throw d.authorization.buildWriteException(obj.getId(), permissions, false);
        }

        // Create the message skeleton
        return new MessageDataDto(header, digest, encPayload);
    }

    public <T extends BaseDao> @Nullable T fromDataMessage(@Nullable MessageDataMetaDto msg, boolean shouldThrow)
    {
        if (msg == null) return null;
        return fromDataMessage(msg.getData(), shouldThrow);
    }
    
    protected <T extends BaseDao> @Nullable T fromDataMessage(@Nullable MessageDataDto msg, boolean shouldThrow)
    {
        if (msg == null || msg.hasPayload() == false) {
            return null;
        }

        T ret = readObjectFromDataMessage(msg, shouldThrow);
        if (ret == null) return null;

        validateObjectAfterRead(ret, msg);

        ret.topicName = d.requestContext.getCurrentTopicScope();
        return ret;
    }

    @SuppressWarnings({"unchecked"})
    private <T extends BaseDao> @Nullable T lintDataObject(@Nullable T _orig, MessageDataDto msg) {
        T orig = _orig;
        if (orig == null) return null;

        Object cloned = d.merger.cloneObject(orig);
        if (cloned == null) return null;
        T ret = (T)cloned;
        ret.version = msg.getHeader().getVersionOrThrow();
        ret.previousVersion = msg.getHeader().getPreviousVersion();
        ret.mergesVersions = msg.getHeader().getMerges();
        return ret;
    }

    @SuppressWarnings({"unchecked"})
    protected <T extends BaseDao> @Nullable T readObjectFromDataMessage(MessageDataDto msg, boolean shouldThrow)
    {
        // We need to decrypt the data using an encryption key, search for it
        // using all the private toPutKeys we have in our token
        byte[] aesKey = getAesKeyForHeader(msg.getHeader(), shouldThrow);
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
            T orig = (T)this.decryptCacheObj.get(cacheKey, () -> {
                BaseDao ret = readObjectFromDataMessageInternal(cacheKey, aesKey, msg);
                if (ret == null) throw new RuntimeException("Failed to deserialize the data object.");
                if (ret instanceof Immutalizable) ((Immutalizable)ret).immutalize();
                return ret;
            });
            return lintDataObject(orig, msg);
        } catch (ExecutionException e) {
            T orig = readObjectFromDataMessageInternal(cacheKey, aesKey, msg);
            return lintDataObject(orig, msg);
        }
    }

    private <T extends BaseDao> byte @Nullable [] readDataFromDataMessageInternal(byte[] aesKey, MessageDataDto msg)
    {
        byte[] encPayloadBytes = msg.getPayloadBytes();
        if (encPayloadBytes == null) return null;
        return d.encryptor.decryptAes(aesKey, encPayloadBytes);
    }

    @SuppressWarnings({"unchecked"})
    private <T extends BaseDao> @Nullable T readObjectFromDataMessageInternal(@Hash String cacheKey, byte[] aesKey, MessageDataDto msg)
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

        // Decrypt the data entity back into its original form and return it
        return (T)d.os.deserializeObj(payloadBytes);
    }

    private <T extends BaseDao> void validateObjectAfterRead(T ret, MessageDataDto msg)
    {
        // The ID must match the header
        MessageDataHeaderDto header = msg.getHeader();
        @DaoId UUID id = header.getIdOrThrow();
        if (id.equals(ret.getId()) == false) {
            throw new RuntimeException("Read access denied (id does not match) - ID=" + id);
        }

        // Make sure the deserialized type matches the header
        if (header.getPayloadClazzOrThrow().equals(ret.getClass().getSimpleName()) == false) {
            throw new RuntimeException("Read access denied (payload types do not match) - ID=" + id);
        }
    }

    private byte @Nullable [] getAesKeyForHeader(MessageDataHeaderDto header, boolean shouldThrow)
    {
        byte[] aesKey = null;
        @Hash String encryptKeyHash = header.getEncryptKeyHash();
        if (encryptKeyHash != null) aesKey = d.encryptKeyCachePerRequest.getEncryptKey(encryptKeyHash);
        if (aesKey == null) {

            if (encryptKeyHash != null) {
                Set<MessagePrivateKeyDto> rights = this.d.currentRights.getRightsRead();
                for (MessagePrivateKeyDto privateKey : rights) {
                    aesKey = d.encryptKeyCachePerRequest.getEncryptKey(encryptKeyHash, privateKey);
                    if (aesKey != null) break;
                }
            }

            if (aesKey == null) {
                if (shouldThrow == true) {
                    EffectivePermissions permissions = d.authorization.perms(header.getIdOrThrow(), header.getParentId(), false);
                    throw d.authorization.buildReadException(header.getIdOrThrow(), permissions, true);
                }
                return null;
            }
        }
        return aesKey;
    }
}
