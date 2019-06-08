package com.tokera.ate.io.core;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageEncryptTextDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.api.ISecureKeyRepository;
import com.tokera.ate.io.repo.DataPartition;
import com.tokera.ate.io.repo.DataPartitionChain;
import com.tokera.ate.units.Hash;
import com.tokera.ate.units.Secret;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.bouncycastle.crypto.InvalidCipherTextException;

import java.io.IOException;

/**
 * Represents the default key store which just stores the encryption keys in the distributed commit
 * log itself (obviously they are also encrypted themselves for security reasons).
 */
public class DefaultSecureKeyRepository implements ISecureKeyRepository {
    private AteDelegate d = AteDelegate.get();

    @Override
    public @Secret byte @Nullable [] get(IPartitionKey partitionKey, @Hash String lookupKey, MessagePrivateKeyDto accessKey) {
        DataPartitionChain chain = d.io.backend().getChain(partitionKey);

        // Loop through all the private toPutKeys that we own and try and find
        // an AES key that was encrypted for it
        try {
            MessageEncryptTextDto text = chain.getEncryptedText(d.encryptor.getPublicKeyHash(accessKey), lookupKey);
            if (text == null) return null;
            byte[] enc = text.getEncryptedTextBytes();

            return d.encryptor.decrypt(accessKey, enc);
        } catch (IOException | InvalidCipherTextException ex) {
            return null;
        }
    }

    @Override
    public void put(IPartitionKey partitionKey, @Hash String lookupKey, @Secret byte[] secretKey, @Hash String publicKeyHash) {
        DataPartition kt = d.io.backend().getPartition(partitionKey);
        DataPartitionChain chain = kt.getChain();

        // Get the public key
        MessagePublicKeyDto publicKey = chain.getPublicKey(publicKeyHash);
        if (publicKey == null) {
            throw new RuntimeException("We encountered a public key [" + publicKeyHash + "] that is not yet known to the distributed commit log. Ensure all public keys are merged before using them in data entities by either calling mergeLater(obj), mergeThreeWay(obj) or mergeThreeWay(publicKeyOrNull).");
        }

        // Encrypt the key
        byte[] encKey = d.encryptor.encrypt(publicKey, secretKey);

        // Create a message and add it
        MessageEncryptTextDto encryptText = new MessageEncryptTextDto(
                publicKeyHash,
                lookupKey,
                encKey);
        kt.write(encryptText, d.genericLogger);
    }

    @Override
    public boolean exists(IPartitionKey partitionKey, @Hash String lookupKey, @Hash String publicKeyHash) {
        DataPartitionChain chain = this.d.storageFactory.get().backend().getChain(partitionKey);
        return chain.getEncryptedText(publicKeyHash, lookupKey) != null;
    }
}
