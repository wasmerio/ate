package com.tokera.ate.security;

import com.tokera.ate.common.MapTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.DataPartitionChain;

import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentMap;
import javax.enterprise.context.RequestScoped;

import com.tokera.ate.units.Hash;
import com.tokera.ate.units.Secret;
import org.checkerframework.checker.nullness.qual.Nullable;

/**
 * Session cache used to find and cache the decryption keys for various hashes for the duration of a currentRights scope
 */
@RequestScoped
public class EncryptKeyCachePerRequest {

    private AteDelegate d = AteDelegate.get();
    
    private final Map<String, byte[]> localEncryptKeyCache = new HashMap<>();
    private final Set<String> nakCache = new HashSet<>();
    private final ConcurrentMap<String, MessagePrivateKeyDto> signKeyCache = new ConcurrentHashMap<>();
    
    /**
     * Gets the private key pair for a particular public key if the caller has
     * access to it in the token
     * @return Byte array that represents the private key or null if the private
     * key does not exist
     */
    public @Secret byte @Nullable [] getEncryptKey(IPartitionKey partitionKey, @Hash String lookupKey)
    {
        /// Check the cache and nak
        Map<String, byte[]> cache = getEncryptKeyCache();
        @Hash byte[] aesKey = MapTools.getOrNull(cache, lookupKey);
        if (aesKey != null) return aesKey;
        if (this.hasNakForSigningKey(lookupKey) == true) {
            return null;
        }
        
        // Get the partition this is related to
        DataPartitionChain chain = this.d.storageFactory.get().backend().getChain(partitionKey);
        
        // Loop through all the private toPutKeys that we own and try and find
        // an AES key that was encrypted for it
        for (MessagePrivateKeyDto key : d.currentRights.getRightsRead()) {
            aesKey = getEncryptKeyInternal(chain, lookupKey, key);
            if (aesKey != null) {
                cache.put(lookupKey, aesKey);
                return aesKey;
            }
        }
        
        // The key does not exist but we should still record that fact
        this.addNakForSigningKey(lookupKey);
        return null;
    }

    private Map<String, byte[]> getEncryptKeyCache()
    {
        if (d.currentToken.getWithinTokenScope()) {
            return d.tokenSecurity.getEncryptKeyCache();
        }
        return this.localEncryptKeyCache;
    }

    /**
     * Gets the private key pair for a particular public key if the caller has
     * access to it in the token
     * @return Byte array that represents the private key or null if the private
     * key does not exist
     */
    public @Secret byte @Nullable [] getEncryptKey(IPartitionKey partitionKey, @Hash String encryptKeyHash, MessagePrivateKeyDto key)
    {
        // Get the partition this is related to
        DataPartitionChain chain = this.d.storageFactory.get().backend().getChain(partitionKey);

        // Return the key
        return getEncryptKeyInternal(chain, encryptKeyHash, key);
    }

    private @Secret byte @Nullable [] getEncryptKeyInternal(DataPartitionChain chain, @Hash String lookupKey, MessagePrivateKeyDto key)
    {
        return d.io.secureKeyResolver().get(chain.partitionKey(), lookupKey, key);
    }

    /**
     * @param encryptKeyHash Hash of the plain text before it was encrypted
     * @param keyPublicKeyHash Public key that was used toe encrypt the plain text
     * @return Returns true if the encrypted test exists or not in the chain of trust
     */
    public boolean hasEncryptKey(IPartitionKey partitionKey, @Hash String encryptKeyHash, @Hash String keyPublicKeyHash)
    {
        return d.io.secureKeyResolver().exists(partitionKey, encryptKeyHash, keyPublicKeyHash);
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