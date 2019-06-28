package com.tokera.ate.io.api;

import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.units.Hash;
import com.tokera.ate.units.Secret;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.UUID;

/**
 * Interface used to getData and set the encryption keys under a particular metadata context
 * (This interface can be used to erase data for compliance or security reasons, e.g. GDPR)
 */
public interface ISecurityCastleFactory {

    /**
     * Gets a secret key based on a public key and a hash of the secret key
     * @param partitionKey The partition that this secure key is related to
     * @param id Lookup identifier of the security boundary
     * @param accessKeys Access keys that can be used to retrieve the secret
     * @return The secret key or null if it can not be found
     */
    @Secret byte @Nullable [] getSecret(IPartitionKey partitionKey, UUID id, Iterable<MessagePrivateKeyDto> accessKeys);

    /**
     * Adds a secret key into the repository
     * @param partitionKey The partition that this secure key is related to
     * @param secret The secret key to be added
     * @param id Lookup identifier of the security boundary
     * @param accessKeys List of the access keys that will be able to getData the secret
     */
    void putSecret(IPartitionKey partitionKey, UUID id, @Secret byte[] secret, Iterable<MessagePublicKeyDto> accessKeys);

    /**
     * @param partitionKey The partition that this secure key is related to
     * @param id Lookup identifier of the security boundary
     * @param publicKeyHash Hash of the public key related to the access key
     * @return Returns true if the encryption key exists in this repository
     */
    boolean exists(IPartitionKey partitionKey, UUID id, @Hash String publicKeyHash);

    /**
     * @param partitionKey The partition that this secure key is related to
     * @param id Lookup identifier of the security boundary
     * @return Returns true if the castle exists at all
     */
    boolean exists(IPartitionKey partitionKey, UUID id);
}
