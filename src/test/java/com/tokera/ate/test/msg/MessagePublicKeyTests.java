package com.tokera.ate.test.msg;

import com.google.common.collect.Lists;
import com.tokera.ate.dao.enumerations.KeyType;
import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.dto.msg.MessageKeyPartDto;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.client.TestTools;
import org.apache.commons.codec.binary.Base64;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.channels.Channels;
import java.nio.channels.WritableByteChannel;
import java.util.Random;

@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public class MessagePublicKeyTests
{
    @Test
    public void forwardTest()
    {
        byte[] bytes1 = new byte[20];
        byte[] bytes2 = new byte[20];
        byte[] bytes3 = new byte[2000];
        new Random().nextBytes(bytes1);
        new Random().nextBytes(bytes2);
        new Random().nextBytes(bytes3);

        MessageKeyPartDto publicPart = new MessageKeyPartDto(KeyType.ntru, 128, Base64.encodeBase64URLSafeString(bytes3));
        MessagePublicKeyDto data = new MessagePublicKeyDto(Lists.newArrayList(publicPart));
        data.setAlias("THEALIAS");
        data.setPublicKeyHash(Base64.encodeBase64URLSafeString(bytes2));
        
        MessagePublicKeyDto data2 = new MessagePublicKeyDto(data.createFlatBuffer());
        MessageKeyPartDto publicPart2 = data2.getPublicParts().stream().findFirst().orElse(null);
        assert publicPart2 != null : "@AssumeAssertion(nullness): Must not be null";

        TestTools.assertEqualAndNotNull(data.getAlias(), data2.getAlias());
        TestTools.assertEqualAndNotNull(publicPart.getKey(), publicPart2.getKey());
        TestTools.assertEqualAndNotNull(publicPart.getKeyBytes(), publicPart2.getKeyBytes());
        data2.setAlias("THEALIAS");
        TestTools.assertEqualAndNotNull(data.getPublicKeyHash(), data2.getPublicKeyHash());
        TestTools.assertEqualAndNotNull(publicPart.getKey(), publicPart2.getKey());
        TestTools.assertEqualAndNotNull(publicPart.getKeyBytes(), publicPart2.getKeyBytes());
        
        MessageBase base = data.createBaseFlatBuffer();
        data2 = new MessagePublicKeyDto(base);
        publicPart2 = data2.getPublicParts().stream().findFirst().orElse(null);
        assert publicPart2 != null : "@AssumeAssertion(nullness): Must not be null";

        TestTools.assertEqualAndNotNull(data.getAlias(), data2.getAlias());
        TestTools.assertEqualAndNotNull(publicPart.getKey(), publicPart2.getKey());
        TestTools.assertEqualAndNotNull(publicPart.getKeyBytes(), publicPart2.getKeyBytes());
        data2.setAlias("THEALIAS");
        TestTools.assertEqualAndNotNull(data.getPublicKeyHash(), data2.getPublicKeyHash());
        TestTools.assertEqualAndNotNull(publicPart.getKey(), publicPart2.getKey());
        TestTools.assertEqualAndNotNull(publicPart.getKeyBytes(), publicPart2.getKeyBytes());
    }
    
    @Test
    public void serializeTest() throws IOException
    {
        byte[] bytes1 = new byte[20];
        byte[] bytes2 = new byte[20];
        byte[] bytes3 = new byte[2000];
        new Random().nextBytes(bytes1);
        new Random().nextBytes(bytes2);
        new Random().nextBytes(bytes3);

        MessageKeyPartDto publicPart = new MessageKeyPartDto(KeyType.ntru, 128, Base64.encodeBase64URLSafeString(bytes3));
        MessagePublicKeyDto data = new MessagePublicKeyDto(Lists.newArrayList(publicPart));
        data.setAlias("THEALIAS");
        data.setPublicKeyHash(Base64.encodeBase64URLSafeString(bytes2));
        
        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        WritableByteChannel channel = Channels.newChannel(stream);
        channel.write(data.createFlatBuffer().getByteBuffer().duplicate());
        
        MessagePublicKeyDto data2 = new MessagePublicKeyDto(data.createFlatBuffer());
        MessageKeyPartDto publicPart2 = data2.getPublicParts().stream().findFirst().orElse(null);
        assert publicPart2 != null : "@AssumeAssertion(nullness): Must not be null";
        
        ByteArrayOutputStream stream2 = new ByteArrayOutputStream();
        WritableByteChannel channel2 = Channels.newChannel(stream2);
        channel2.write(data2.createFlatBuffer().getByteBuffer().duplicate());

        TestTools.assertEqualAndNotNull(data.getAlias(), data2.getAlias());
        TestTools.assertEqualAndNotNull(publicPart.getKey(), publicPart2.getKey());
        TestTools.assertEqualAndNotNull(publicPart.getKeyBytes(), publicPart2.getKeyBytes());
        
        byte[] streamBytes1 = stream.toByteArray();
        byte[] streamBytes2 = stream2.toByteArray();
        TestTools.assertEqualAndNotNull(streamBytes1, streamBytes2);
    }
}
