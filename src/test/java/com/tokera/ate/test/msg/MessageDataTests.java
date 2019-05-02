package com.tokera.ate.test.msg;

import com.google.common.base.Objects;
import com.tokera.ate.dao.msg.MessageBase;
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
public class MessageDataTests
{
    
    @Test
    public void forwardTest()
    {
        UUID id = UUID.randomUUID();
        UUID version = UUID.randomUUID();
        UUID previousVersion = UUID.randomUUID();
        byte[] bytes1 = new byte[20];
        byte[] bytes2 = new byte[2000];
        new Random().nextBytes(bytes1);
        new Random().nextBytes(bytes2);
        UUID merge1 = UUID.randomUUID();
        UUID merge2 = UUID.randomUUID();

        MessageDataDigestDto digest = new MessageDataDigestDto(
                "",
                "",
                "",
                Base64.encodeBase64URLSafeString(bytes1)
        );

        MessageDataHeaderDto header = new MessageDataHeaderDto(
                id,
                version,
                previousVersion,
                MyAccount.class
        );
        header.getMerges().add(merge1);
        header.getMerges().add(merge2);

        MessageDataDto data = new MessageDataDto(header, digest, bytes2);
        
        MessageDataDto data2 = new MessageDataDto(data.createFlatBuffer());

        byte[] payload1 = data.getPayloadBytes();
        byte[] payload2 = data2.getPayloadBytes();
        assert payload1 != null : "@AssumeAssertion(nullness): Payload must not be null";
        assert payload2 != null : "@AssumeAssertion(nullness): Payload must not be null";
        Assertions.assertArrayEquals(payload1, payload2);

        data2.setPayloadBytes(bytes2);
        Assertions.assertTrue(Objects.equal(data.getHeader().getIdOrThrow(), data2.getHeader().getIdOrThrow()), "ID is not equal");
        Assertions.assertTrue(Objects.equal(data.getHeader().getVersionOrThrow(), data2.getHeader().getVersionOrThrow()), "Version is not equal");
        Assertions.assertTrue(Objects.equal(data.getHeader().getPreviousVersion(), data2.getHeader().getPreviousVersion()), "Previous Version is not equal");
        Assertions.assertTrue(data.getHeader().getMerges().size() == 2, "Merge versions are missing");
        Assertions.assertTrue(data.getHeader().getMerges().contains(merge1), "Merge versions is missing merge1 value");
        Assertions.assertTrue(data.getHeader().getMerges().contains(merge2), "Merge versions is missing merge2 value");

        MessageDataDigestDto digest1 = digest;
        MessageDataDigestDto digest2 = data2.getDigest();
        assert digest2 != null : "@AssumeAssertion(nullness): Digest must not be null";
        Assertions.assertNotNull(digest1, "Digest is null");
        Assertions.assertNotNull(digest2, "Digest is null");
        Assertions.assertTrue(Objects.equal(digest1.getPublicKeyHash(), digest2.getPublicKeyHash()), "Public key hash is not equal");
        
        MessageBase base = data.createBaseFlatBuffer();
        data2 = new MessageDataDto(base);

        payload1 = data.getPayloadBytes();
        payload2 = data2.getPayloadBytes();
        assert payload1 != null : "@AssumeAssertion(nullness): Payload must not be null";
        assert payload2 != null : "@AssumeAssertion(nullness): Payload must not be null";
        Assertions.assertNotNull(payload1);
        Assertions.assertNotNull(payload2);
        Assertions.assertArrayEquals(payload1, payload2);

        data2.setPayloadBytes(bytes2);
        Assertions.assertTrue(Objects.equal(data.getHeader().getIdOrThrow(), data2.getHeader().getIdOrThrow()), "ID is not equal");
        Assertions.assertTrue(Objects.equal(data.getHeader().getVersionOrThrow(), data2.getHeader().getVersionOrThrow()), "Version is not equal");
        Assertions.assertTrue(Objects.equal(data.getHeader().getPreviousVersion(), data2.getHeader().getPreviousVersion()), "Previous Version is not equal");
        Assertions.assertTrue( data.getHeader().getMerges().size() == 2, "Merge versions are missing");
        Assertions.assertTrue(data.getHeader().getMerges().contains(merge1), "Merge versions is missing merge1 value");
        Assertions.assertTrue(data.getHeader().getMerges().contains(merge2), "Merge versions is missing merge2 value");

        digest1 = digest;
        digest2 = data2.getDigest();
        assert digest2 != null : "@AssumeAssertion(nullness): Digest must not be null";
        Assertions.assertNotNull(digest2);
        Assertions.assertTrue(Objects.equal(digest1.getPublicKeyHash(), digest2.getPublicKeyHash()), "Public key hash is not equal");
    }
    
    @Test
    public void serializeTest() throws IOException
    {
        UUID id = UUID.randomUUID();
        byte[] bytes1 = new byte[20];
        byte[] bytes2 = new byte[2000];
        new Random().nextBytes(bytes1);
        new Random().nextBytes(bytes2);

        MessageDataDigestDto digest = new MessageDataDigestDto(
                "",
                "ABA",
                "ABC",
                Base64.encodeBase64URLSafeString(bytes1)
        );

        MessageDataHeaderDto header = new MessageDataHeaderDto(
                id,
                UUID.randomUUID(),
                null,
                MyAccount.class
        );

        MessageDataDto data = new MessageDataDto(header, digest, bytes2);

        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        WritableByteChannel channel = Channels.newChannel(stream);
        channel.write(data.createFlatBuffer().getByteBuffer().duplicate());

        MessageDataDto data2 = new MessageDataDto(data.createFlatBuffer());

        ByteArrayOutputStream stream2 = new ByteArrayOutputStream();
        WritableByteChannel channel2 = Channels.newChannel(stream2);
        channel2.write(data2.createFlatBuffer().getByteBuffer().duplicate());

        byte[] payload1 = data.getPayloadBytes();
        byte[] payload2 = data2.getPayloadBytes();
        assert payload1 != null : "@AssumeAssertion(nullness): Payload must not be null";
        assert payload2 != null : "@AssumeAssertion(nullness): Payload must not be null";
        Assertions.assertNotNull(payload1, "Payload is null");
        Assertions.assertNotNull(payload2, "Payload is null");
        Assertions.assertArrayEquals(payload1, payload2);

        Assertions.assertTrue(Objects.equal(data.getHeader().getIdOrThrow(), data2.getHeader().getIdOrThrow()), "ID is not equal");

        MessageDataDigestDto digest1 = digest;
        MessageDataDigestDto digest2 = data2.getDigest();
        assert digest2 != null : "@AssumeAssertion(nullness): Digest must not be null";
        Assertions.assertNotNull(digest2, "Digest is null");
        Assertions.assertTrue(Objects.equal(digest1.getPublicKeyHash(), digest2.getPublicKeyHash()), "Public key hash is not equal");

        byte[] streamBytes1 = stream.toByteArray();
        byte[] streamBytes2 = stream2.toByteArray();
        Assertions.assertArrayEquals(streamBytes1, streamBytes2);
    }
}
