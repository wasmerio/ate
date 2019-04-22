package com.tokera.ate.test.msg;

import com.google.api.client.util.Base64;
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
import junit.framework.Assert;
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
                MyAccount.class.getSimpleName()
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
        Assert.assertTrue("ID is not equal", Objects.equal(data.getHeader().getIdOrThrow(), data2.getHeader().getIdOrThrow()));
        Assert.assertTrue("Version is not equal", Objects.equal(data.getHeader().getVersionOrThrow(), data2.getHeader().getVersionOrThrow()));
        Assert.assertTrue("Previous Version is not equal", Objects.equal(data.getHeader().getPreviousVersion(), data2.getHeader().getPreviousVersion()));
        Assert.assertTrue( "Merge versions are missing", data.getHeader().getMerges().size() == 2);
        Assert.assertTrue( "Merge versions is missing merge1 value", data.getHeader().getMerges().contains(merge1));
        Assert.assertTrue( "Merge versions is missing merge2 value", data.getHeader().getMerges().contains(merge2));

        MessageDataDigestDto digest1 = digest;
        MessageDataDigestDto digest2 = data2.getDigest();
        assert digest2 != null : "@AssumeAssertion(nullness): Digest must not be null";
        Assert.assertNotNull("Digest is null", digest1);
        Assert.assertNotNull("Digest is null", digest2);
        Assert.assertTrue("Public key hash is not equal", Objects.equal(digest1.getPublicKeyHash(), digest2.getPublicKeyHash()));
        
        MessageBase base = data.createBaseFlatBuffer();
        data2 = new MessageDataDto(base);

        payload1 = data.getPayloadBytes();
        payload2 = data2.getPayloadBytes();
        assert payload1 != null : "@AssumeAssertion(nullness): Payload must not be null";
        assert payload2 != null : "@AssumeAssertion(nullness): Payload must not be null";
        Assert.assertNotNull(payload1);
        Assert.assertNotNull(payload2);
        Assertions.assertArrayEquals(payload1, payload2);

        data2.setPayloadBytes(bytes2);
        Assert.assertTrue("ID is not equal", Objects.equal(data.getHeader().getIdOrThrow(), data2.getHeader().getIdOrThrow()));
        Assert.assertTrue("Version is not equal", Objects.equal(data.getHeader().getVersionOrThrow(), data2.getHeader().getVersionOrThrow()));
        Assert.assertTrue("Previous Version is not equal", Objects.equal(data.getHeader().getPreviousVersion(), data2.getHeader().getPreviousVersion()));
        Assert.assertTrue( "Merge versions are missing", data.getHeader().getMerges().size() == 2);
        Assert.assertTrue( "Merge versions is missing merge1 value", data.getHeader().getMerges().contains(merge1));
        Assert.assertTrue( "Merge versions is missing merge2 value", data.getHeader().getMerges().contains(merge2));

        digest1 = digest;
        digest2 = data2.getDigest();
        assert digest2 != null : "@AssumeAssertion(nullness): Digest must not be null";
        Assert.assertNotNull("Digest is null", digest2);
        Assert.assertTrue("Public key hash is not equal", Objects.equal(digest1.getPublicKeyHash(), digest2.getPublicKeyHash()));
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
                "Accoount"
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
        Assert.assertNotNull("Payload is null", payload1);
        Assert.assertNotNull("Payload is null", payload2);
        Assertions.assertArrayEquals(payload1, payload2);

        Assert.assertTrue("ID is not equal", Objects.equal(data.getHeader().getIdOrThrow(), data2.getHeader().getIdOrThrow()));

        MessageDataDigestDto digest1 = digest;
        MessageDataDigestDto digest2 = data2.getDigest();
        assert digest2 != null : "@AssumeAssertion(nullness): Digest must not be null";
        Assert.assertNotNull("Digest is null", digest2);
        Assert.assertTrue("Public key hash is not equal", Objects.equal(digest1.getPublicKeyHash(), digest2.getPublicKeyHash()));

        byte[] streamBytes1 = stream.toByteArray();
        byte[] streamBytes2 = stream2.toByteArray();
        Assertions.assertArrayEquals(streamBytes1, streamBytes2);
    }
}
