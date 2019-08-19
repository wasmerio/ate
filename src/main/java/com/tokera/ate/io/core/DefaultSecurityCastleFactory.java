package com.tokera.ate.io.core;

import com.google.common.cache.Cache;
import com.google.common.cache.CacheBuilder;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.dto.msg.MessageSecurityCastleDto;
import com.tokera.ate.dto.msg.MessageSecurityGateDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.api.ISecurityCastleFactory;
import com.tokera.ate.io.repo.DataPartition;
import com.tokera.ate.io.repo.DataPartitionChain;
import com.tokera.ate.units.Hash;
import com.tokera.ate.units.Secret;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.bouncycastle.crypto.InvalidCipherTextException;

import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.Response;
import java.io.IOException;
import java.util.UUID;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;

/**
 * Represents the default key store which just stores the encryption keys in the distributed commit
 * log itself (obviously they are also encrypted themselves for security reasons).
 */
public class DefaultSecurityCastleFactory implements ISecurityCastleFactory {
    private AteDelegate d = AteDelegate.get();

    private static Cache<String, byte[]> secretCache = CacheBuilder.newBuilder()
            .maximumSize(20000)
            .expireAfterWrite(5, TimeUnit.MINUTES)
            .build();

    @Override
    public @Nullable @Secret byte[] getSecret(IPartitionKey partitionKey, UUID id, Iterable<MessagePrivateKeyDto> accessKeys) {
        DataPartitionChain chain = d.io.backend().getChain(partitionKey);

        // Loop through all the private toPutKeys that we own and try and find
        // an AES key that was encrypted for it
        MessageSecurityCastleDto castle = chain.getCastle(id);
        if (castle == null) return null;

        for (MessagePrivateKeyDto accessKey : accessKeys) {
            String encStr = MapTools.getOrNull(castle.getLookup(), accessKey.getPublicKeyHash());
            if (encStr == null) continue;

            String lookup = accessKey.getPrivateKeyHash() + encStr;
            try {
                return secretCache.get(lookup, () -> d.encryptor.decrypt(accessKey, Base64.decodeBase64(encStr)));
            } catch (ExecutionException e) {
                throw new WebApplicationException("Failed to retrieve AES secret [castle=" + castle.getIdOrThrow() + ", key=" + accessKey.getPublicKeyHash() + "] while processing data object [id=" + partitionKey + ":" + id + "].", e, Response.Status.UNAUTHORIZED);
            }
        }
        return null;
    }

    @Override
    public void putSecret(IPartitionKey partitionKey, UUID id, @Secret byte[] secret, Iterable<MessagePublicKeyDto> accessKeys) {
        DataPartition kt = d.io.backend().getPartition(partitionKey);

        // Create a new castle
        MessageSecurityCastleDto castle = new MessageSecurityCastleDto(id);

        // Add the encryption parts
        for (MessagePublicKeyDto publicKey : accessKeys) {
            byte[] encKey = d.encryptor.encrypt(publicKey, secret);
            castle.getGates().add(new MessageSecurityGateDto(publicKey.getPublicKeyHash(), encKey));
        }

        // Write it to the partition
        kt.write(castle, d.genericLogger);
    }

    @Override
    public boolean exists(IPartitionKey partitionKey, UUID id, @Hash String publicKeyHash) {
        DataPartitionChain chain = this.d.storageFactory.get().backend().getChain(partitionKey);
        MessageSecurityCastleDto castle = chain.getCastle(id);
        if (castle == null) return false;
        return castle.getLookup().containsKey(publicKeyHash);
    }

    @Override
    public boolean exists(IPartitionKey partitionKey, UUID id) {
        DataPartitionChain chain = this.d.storageFactory.get().backend().getChain(partitionKey);
        return chain.getCastle(id) != null;
    }
}
