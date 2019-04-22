package com.tokera.ate.test.msg;

import com.google.common.base.Objects;
import com.tokera.ate.dao.msg.MessageDataDigest;
import com.tokera.ate.dto.msg.MessageDataDigestDto;
import com.tokera.ate.dto.msg.MessageDataDto;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.channels.Channels;
import java.nio.channels.WritableByteChannel;
import java.util.Random;
import java.util.UUID;

import com.tokera.ate.dto.msg.MessageDataHeaderDto;
import com.tokera.ate.test.dao.MyAccount;
import org.apache.commons.codec.binary.Base64;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;

@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public class MessageDataDigestTests
{
    @Test
    public void forwardTest()
    {
        byte[] bytes1 = new byte[20];
        byte[] bytes2 = new byte[20];
        byte[] bytes3 = new byte[20];
        byte[] bytes4 = new byte[20];
        new Random().nextBytes(bytes1);
        new Random().nextBytes(bytes2);
        new Random().nextBytes(bytes3);
        new Random().nextBytes(bytes4);
        
        MessageDataDigestDto header = new MessageDataDigestDto(
                Base64.encodeBase64URLSafeString(bytes1),
                Base64.encodeBase64URLSafeString(bytes4),
                Base64.encodeBase64URLSafeString(bytes3),
                Base64.encodeBase64URLSafeString(bytes2)
        );
        
        MessageDataDigest digest = header.createFlatBuffer();
        MessageDataDigestDto header2 = new MessageDataDigestDto(digest);

        Assertions.assertTrue(Objects.equal(header.getSeed(), header2.getSeed()), "Seed is not equal");
        header2.setSeed(Base64.encodeBase64URLSafeString(bytes1));
        Assertions.assertTrue(Objects.equal(header.getPublicKeyHash(), header2.getPublicKeyHash()), "Public key hash is not equal");
        Assertions.assertTrue(Objects.equal(header.getDigest(), header2.getDigest()), "Digest is not equal");
        Assertions.assertTrue(Objects.equal(header.getSignature(), header2.getSignature()), "Signature is not equal");
    }
    
    @Test
    public void serializeTest() throws IOException
    {
        byte[] bytes1 = new byte[20];
        byte[] bytes2 = new byte[20];
        byte[] bytes3 = new byte[20];
        byte[] bytes4 = new byte[20];
        new Random().nextBytes(bytes1);
        new Random().nextBytes(bytes2);
        new Random().nextBytes(bytes3);
        new Random().nextBytes(bytes4);
        
        MessageDataDigestDto digest = new MessageDataDigestDto(
                Base64.encodeBase64URLSafeString(bytes1),
                Base64.encodeBase64URLSafeString(bytes4),
                Base64.encodeBase64URLSafeString(bytes3),
                Base64.encodeBase64URLSafeString(bytes2)
        );

        MessageDataHeaderDto header = new MessageDataHeaderDto(
                UUID.randomUUID(),
                UUID.randomUUID(),
                null,
                MyAccount.class.getSimpleName()
        );
        
        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        WritableByteChannel channel = Channels.newChannel(stream);
        channel.write(digest.createFlatBuffer().getByteBuffer().duplicate());
        
        MessageDataDto data = new MessageDataDto(header, digest, null);
        
        MessageDataDto data2 = new MessageDataDto(data.createFlatBuffer());
        MessageDataDigestDto header2 = data2.getDigest();
        assert header2 != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(header2);
        
        ByteArrayOutputStream stream2 = new ByteArrayOutputStream();
        WritableByteChannel channel2 = Channels.newChannel(stream2);
        channel2.write(header2.createFlatBuffer().getByteBuffer().duplicate());

        Assertions.assertTrue(Objects.equal(digest.getSeed(), header2.getSeed()), "Seed is not equal");
        Assertions.assertTrue(Objects.equal(digest.getPublicKeyHash(), header2.getPublicKeyHash()), "Public key hash is not equal");
        Assertions.assertTrue(Objects.equal(digest.getDigest(), header2.getDigest()), "Digest is not equal");
        Assertions.assertTrue(Objects.equal(digest.getSignature(), header2.getSignature()), "Signature is not equal");
        
        byte[] streamBytes1 = stream.toByteArray();
        byte[] streamBytes2 = stream2.toByteArray();
        Assertions.assertArrayEquals(streamBytes1, streamBytes2);
    }
}
