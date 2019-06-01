package com.tokera.ate.io.repo;

import com.tokera.ate.scopes.Startup;
import com.tokera.ate.dao.kafka.MessageSerializer;
import com.tokera.ate.dto.EffectivePermissions;
import com.tokera.ate.dto.msg.MessageDataDigestDto;
import com.tokera.ate.dto.msg.MessageDataHeaderDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.security.EncryptKeyCachePerRequest;
import com.tokera.ate.security.Encryptor;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;

import com.tokera.ate.units.Hash;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.Nullable;

/**
 * Builds a crypto signature of the data message payload so that it can be validated without actually reading the data
 */
@Startup
@ApplicationScoped
public class DataSignatureBuilder
{
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private Encryptor encryptor;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private EncryptKeyCachePerRequest encryptSession;

    private byte[] generateStreamBytes(MessageDataHeaderDto header, byte @Nullable [] payloadBytes) {

        // Start preparing the digest bytes
        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        MessageSerializer.writeBytes(stream, header.createFlatBuffer());

        // Embed the payload if one exists
        if (payloadBytes != null)
        {
            // Add it to the digest entropy
            try {
                stream.write(payloadBytes);
            } catch (IOException ex) {
                throw new RuntimeException(ex);
            }
        }

        byte[] streamBytes = stream.toByteArray();
        return streamBytes;
    }

    private MessageDataDigestDto generateVerifiedSignature(byte[] streamBytes, MessagePrivateKeyDto privateKey) {

        // Compute the message digest using the newly generated seed
        String seed = encryptor.generateSecret64(128);
        byte[] seedBytes = Base64.decodeBase64(seed);
        byte[] digestBytes = encryptor.hashSha(seedBytes, streamBytes);
        String digest = Base64.encodeBase64URLSafeString(digestBytes);

        // Compute the signature of the digest and verify that it works properly
        @Hash String keyHash = privateKey.getPublicKeyHash();
        if (keyHash == null) throw new RuntimeException("No public hash attached.");

        byte[] sigBytes = encryptor.sign(privateKey, digestBytes);
        String sig = Base64.encodeBase64URLSafeString(sigBytes);
        if (encryptor.verify(privateKey, Base64.decodeBase64(digest), Base64.decodeBase64(sig)) == false) {
            throw new RuntimeException("Failed to verify the key.");
        }

        // Create the signature and return success
        return new MessageDataDigestDto(seed, sig, digest, keyHash);
    }
    
    public @Nullable MessageDataDigestDto signDataMessage(MessageDataHeaderDto header, byte @Nullable [] payloadBytes, EffectivePermissions permissions)
    {
        // Loop through all the roles until we find a key that we can
        // use of writing a valid digest for this entity
        byte[] streamBytes = generateStreamBytes(header, payloadBytes);
        for (String publicKeyHash : permissions.rolesWrite)
        {
            // We can only sign if we have a private key for the pair
            MessagePrivateKeyDto privateKey = encryptSession.getSignKey(publicKeyHash);
            if (privateKey == null) continue;
            
            //LOG.info("ntru-encrypt:\n" + "  - private-key: " + Base64.encodeBase64URLSafeString(privateKey) + "\n  - data: " + Base64.encodeBase64URLSafeString(digestBytes) + "\n");

            // Enter a retrying digest calculation loop that is tollerant to
            // failures to generate a proper digest
            for (int n = 0;; n++)
            {
                try {
                    return generateVerifiedSignature(streamBytes, privateKey);
                } catch (Exception ex) {
                    if (n < 15) continue;
                    throw new RuntimeException(ex);
                }
            }
        }
        
        // Nothing was signed
        return null;
    }
}
