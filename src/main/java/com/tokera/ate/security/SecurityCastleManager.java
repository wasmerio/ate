package com.tokera.ate.security;

import com.tokera.ate.common.MapTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.EffectivePermissions;
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.DataPartitionChain;

import java.util.*;
import javax.enterprise.context.RequestScoped;

import com.tokera.ate.io.repo.DataTransaction;
import com.tokera.ate.providers.PartitionKeySerializer;
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

    /**
     * @return Hash that represents a unique set of read permissions
     */
    public String computePermissionsHash(EffectivePermissions permissions) {
        String seed = new PartitionKeySerializer().write(permissions.partitionKey);
        return d.encryptor.hashShaAndEncode(seed, permissions.rolesRead);
    }

    /**
     * @return Hash that represents a unique set of read permissions
     */
    public String computePermissionsHash(IPartitionKey key, List<String> roles) {
        String seed = new PartitionKeySerializer().write(key);
        return d.encryptor.hashShaAndEncode(seed, roles);
    }

    /**
     * @param partitionKey Partition key that the public keys exist within
     * @param hashes List of public key hashes that will be looked up in the partition
     * @return Set of public keys that can be used for encrypting or signing data
     */
    public Set<MessagePublicKeyDto> findPublicKeys(IPartitionKey partitionKey, Iterable<String> hashes) {
        DataPartitionChain chain = d.io.backend().getChain(partitionKey, true);
        Set<MessagePublicKeyDto> ret = new HashSet<>();
        for (String publicKeyHash : hashes)
        {
            MessagePublicKeyDto publicKey = d.requestContext.currentTransaction().findPublicKey(partitionKey, publicKeyHash);
            if (publicKey != null) {
                ret.add(publicKey);
                continue;
            }

            PrivateKeyWithSeedDto rightsKey = d.currentRights.findKey(publicKeyHash);
            if (rightsKey != null) {
                ret.add(new MessagePublicKeyDto(rightsKey));
                continue;
            }

            publicKey = chain.getPublicKey(publicKeyHash);
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
     * @param roles Set of read permissions that represent the boundary of this castle
     * @return Castle that can be used to encrypt data and later read by anyone who holds the private keys of the read roles
     */
    public SecurityCastleContext makeCastle(IPartitionKey partitionKey, List<String> roles)
    {
        String hash = computePermissionsHash(partitionKey, roles);

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
                                        findPublicKeys(partitionKey, roles));

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
    public @Nullable SecurityCastleContext enterCastle(IPartitionKey partitionKey, UUID castleId, Set<PrivateKeyWithSeedDto> accessKeys)
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
     * @param partitionKey Partition that the castle was previous created within
     * @param castleId ID of the castle that uniquely identifies it
     * @return Returns true if the castle itself exists at all
     */
    public boolean hasCastle(IPartitionKey partitionKey, UUID castleId)
    {
        return d.io.securityCastleFactory().exists(partitionKey, castleId);
    }
}