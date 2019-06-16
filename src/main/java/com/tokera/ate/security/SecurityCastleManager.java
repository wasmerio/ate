package com.tokera.ate.security;

import com.tokera.ate.common.MapTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.EffectivePermissions;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.dto.msg.MessageSecurityCastleDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.api.ISecurityCastleFactory;
import com.tokera.ate.io.repo.DataPartitionChain;

import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentMap;
import javax.enterprise.context.RequestScoped;

import com.tokera.ate.units.Hash;
import com.tokera.ate.units.Secret;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.Nullable;

/**
 * Session cache used to find and cache the decryption keys for various hashes for the duration of a currentRights scope
 */
@RequestScoped
public class SecurityCastleManager {
    private AteDelegate d = AteDelegate.get();

    private final Map<String, SecurityCastleContext> localCastles = new HashMap<>();
    private final Map<UUID, SecurityCastleContext> lookupCastles = new HashMap<>();
    private final Set<String> nakCache = new HashSet<>();
    private final ConcurrentMap<String, MessagePrivateKeyDto> signKeyCache = new ConcurrentHashMap<>();

    /**
     * @return Hash that represents a unique set of read permissions
     */
    public String computePermissionsHash(EffectivePermissions permissions) {
        return d.encryptor.hashShaAndEncode(permissions.rolesRead);
    }

    /**
     * @param partitionKey Partition key that the public keys exist within
     * @param hashes List of public key hashes that will be looked up in the partition
     * @return Set of public keys that can be used for encrypting or signing data
     */
    public Set<MessagePublicKeyDto> findPublicKeys(IPartitionKey partitionKey, Iterable<String> hashes) {
        DataPartitionChain chain = d.io.backend().getChain(partitionKey);
        Set<MessagePublicKeyDto> ret = new HashSet<>();
        for (String publicKeyHash : hashes) {
            MessagePublicKeyDto publicKey = chain.getPublicKey(publicKeyHash);
            if (publicKey == null) {
                throw new RuntimeException("We encountered a public key [" + publicKeyHash + "] that is not yet known to the distributed commit log. Ensure all public keys are merged before using them in data entities by either calling mergeLater(obj), mergeThreeWay(obj) or mergeThreeWay(publicKeyOrNull).");
            }
            ret.add(publicKey);
        }
        return ret;
    }

    /**
     * Creates a security castle for the current request context that matches a particular set of read permissions
     * @param partitionKey Partition key that the castle will be created within
     * @param permissions Set of read permissions that represent the boundary of this castle
     * @return Castle that can be used to encrypt data and later read by anyone who holds the private keys of the read roles
     */
    public SecurityCastleContext makeCastle(IPartitionKey partitionKey, EffectivePermissions permissions)
    {
        String hash = computePermissionsHash(permissions);

        // Perhaps we can reuse a context that already exists in memory
        if (d.bootstrapConfig.getDefaultAutomaticKeyRotation() == false) {
            SecurityCastleContext ret = MapTools.getOrNull(localCastles, hash);
            if (ret != null) return ret;
        }

        // Lets create a new castle for the request context (or reuse one if one is already started in this request)
        return localCastles.computeIfAbsent(hash,
                h ->
                {
                    UUID castleId = UUID.randomUUID();
                    @Secret byte[] key = Base64.decodeBase64(d.encryptor.generateSecret64());
                    d.io.securityCastleFactory()
                            .putSecret( partitionKey,
                                        castleId,
                                        key,
                                        findPublicKeys(partitionKey, permissions.rolesRead));

                    SecurityCastleContext ret = new SecurityCastleContext(castleId, key);
                    lookupCastles.put(castleId, ret);
                    return ret;
                });
    }

    /**
     * Enters a security castle that was previous used with the supplied read access rights
     * @param partitionKey Partition that the castle was previous created within
     * @param castleId ID of the castle that uniquely identifies it
     * @param accessKeys List of private access keys that can be used to enter the castle
     * @return Reference to castle context that allows the decryption of data previously saved
     */
    public @Nullable SecurityCastleContext enterCastle(IPartitionKey partitionKey, UUID castleId, Iterable<MessagePrivateKeyDto> accessKeys)
    {
        SecurityCastleContext ret = MapTools.getOrNull(this.lookupCastles, castleId);
        if (ret != null) return ret;

        @Secret byte[] key = d.io.securityCastleFactory().getSecret(partitionKey, castleId, accessKeys);
        if (key == null) return null;

        ret = new SecurityCastleContext(castleId, key);
        lookupCastles.put(castleId, ret);
        return ret;
    }

    /**
     * @param partitionKey Partition that the castle was previous created within
     * @param castleId ID of the castle that uniquely identifies it
     * @param keyPublicKeyHash Public key that was used toe encrypt the plain text
     * @return Returns true if the encrypted test exists or not in the chain of trust
     */
    public boolean hasEncryptKey(IPartitionKey partitionKey, UUID castleId, @Hash String keyPublicKeyHash)
    {
        return d.io.securityCastleFactory().exists(partitionKey, castleId, keyPublicKeyHash);
    }

    /**
     * Adds a NAK to the signing key cache
     */
    private void addNakForSigningKey(String val) {
        nakCache.add(val);
    }

    /**
     * Checks if the signing key cache has a NAK already
     */
    private boolean hasNakForSigningKey(String val) {
        return nakCache.contains(val);
    }

    /**
     * Gets the private key pair for a particular public key if the caller has
     * access to it in the token
     * @param publicKeyHash String that represents the public key
     * @return Byte array that represents the private key or null if the private
     * key does not exist
     */
    public @Nullable MessagePrivateKeyDto getSignKey(String publicKeyHash)
    {
        // Check the cache
        MessagePrivateKeyDto signKey = null;
        if (signKeyCache.containsKey(publicKeyHash)) {
            signKey = signKeyCache.get(publicKeyHash);
        }
        if (signKey != null) return signKey;

        // Check the null key cache (this speeds things up alot)
        if (this.hasNakForSigningKey(publicKeyHash) == true) {
            return null;
        }

        // Loop through all the private toPutKeys that we own and try and find
        // an AES key that was encrypted for it
        MessagePrivateKeyDto key = d.currentRights.getRightsWrite()
                .stream()
                .filter(p -> publicKeyHash.equals(d.encryptor.getPublicKeyHash(p)))
                .findFirst()
                .orElse(null);
        if (key != null) {
            this.signKeyCache.put(publicKeyHash, key);
            return key;
        }

        // The key does not exist but we should still record that fact
        this.addNakForSigningKey(publicKeyHash);
        return null;
    }

    public void addSignKeyToCache(String publicKeyHash, MessagePrivateKeyDto key) {
        this.signKeyCache.put(publicKeyHash, key);
    }
}