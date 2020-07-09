package com.tokera.ate.test.msg;

import com.google.common.collect.Lists;
import com.tokera.ate.dao.enumerations.KeyType;
import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.dao.msg.MessagePublicKey;
import com.tokera.ate.dto.msg.MessageKeyPartDto;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.client.TestTools;
import java.util.Collections;
import org.apache.commons.codec.binary.Base64;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.channels.Channels;
import java.nio.channels.WritableByteChannel;
import java.util.Random;

@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public class MessagePublicKeyTests
{
    @Test
    public void forwardTest()
    {
        Random random = new Random();
        byte[] bytes1 = new byte[20];
        byte[] bytes2 = new byte[20];
        byte[] bytes3 = new byte[2000];
        byte[] bytes4 = new byte[3000];
        random.nextBytes(bytes1);
        random.nextBytes(bytes2);
        random.nextBytes(bytes3);
        random.nextBytes(bytes4);

        MessageKeyPartDto publicPart = new MessageKeyPartDto(KeyType.ntru, 128, Base64.encodeBase64URLSafeString(bytes3));
        MessagePublicKeyDto data = new MessagePublicKeyDto(Collections.singletonList(publicPart));
        data.setAlias("THEALIAS");
        data.setPublicKeyHash(Base64.encodeBase64URLSafeString(bytes2));
        
        MessagePublicKeyDto data2 = new MessagePublicKeyDto(data.createFlatBuffer());
        MessageKeyPartDto publicPart2 = data2.getPublicParts().stream().findFirst().orElse(null);
        assert publicPart2 != null : "@AssumeAssertion(nullness): Must not be null";

        TestTools.assertEqualAndNotNull(data.getAlias(), data2.getAlias());
        TestTools.assertEqualAndNotNull(publicPart.getKey64(), publicPart2.getKey64());
        TestTools.assertEqualAndNotNull(publicPart.getKeyBytes(), publicPart2.getKeyBytes());
        data2.setAlias("THEALIAS");
        TestTools.assertEqualAndNotNull(data.getPublicKeyHash(), data2.getPublicKeyHash());
        TestTools.assertEqualAndNotNull(publicPart.getKey64(), publicPart2.getKey64());
        TestTools.assertEqualAndNotNull(publicPart.getKeyBytes(), publicPart2.getKeyBytes());
        
        MessageBase base = data.createBaseFlatBuffer();
        data2 = new MessagePublicKeyDto(base);
        publicPart2 = data2.getPublicParts().stream().findFirst().orElse(null);
        assert publicPart2 != null : "@AssumeAssertion(nullness): Must not be null";

        TestTools.assertEqualAndNotNull(data.getAlias(), data2.getAlias());
        TestTools.assertEqualAndNotNull(publicPart.getKey64(), publicPart2.getKey64());
        TestTools.assertEqualAndNotNull(publicPart.getKeyBytes(), publicPart2.getKeyBytes());
        data2.setAlias("THEALIAS");
        TestTools.assertEqualAndNotNull(data.getPublicKeyHash(), data2.getPublicKeyHash());
        TestTools.assertEqualAndNotNull(publicPart.getKey64(), publicPart2.getKey64());
        TestTools.assertEqualAndNotNull(publicPart.getKeyBytes(), publicPart2.getKeyBytes());
    }
    
    @Test
    public void serializeTest() throws IOException
    {
        Random random = new Random();
        byte[] bytes1 = new byte[20];
        byte[] bytes2 = new byte[20];
        byte[] bytes3 = new byte[2000];
        byte[] bytes4 = new byte[3000];
        random.nextBytes(bytes1);
        random.nextBytes(bytes2);
        random.nextBytes(bytes3);
        random.nextBytes(bytes4);

        MessageKeyPartDto publicPartA = new MessageKeyPartDto(KeyType.ntru, 128, Base64.encodeBase64URLSafeString(bytes3));
        MessageKeyPartDto publicPartB = new MessageKeyPartDto(KeyType.aes, 128, Base64.encodeBase64URLSafeString(bytes4));
        MessagePublicKeyDto data = new MessagePublicKeyDto(Lists.newArrayList(publicPartA, publicPartB));
        data.setAlias("THEALIAS");
        data.setPublicKeyHash(Base64.encodeBase64URLSafeString(bytes2));
        
        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        WritableByteChannel channel = Channels.newChannel(stream);
        channel.write(data.createFlatBuffer().getByteBuffer().duplicate());
        
        MessagePublicKeyDto data2 = new MessagePublicKeyDto(data.createFlatBuffer());
        MessageKeyPartDto publicPartA2 = data2.getPublicParts().stream().findFirst().orElse(null);
        MessageKeyPartDto publicPartB2 = data2.getPublicParts().stream().skip(1).findFirst().orElse(null);
        assert publicPartA2 != null : "@AssumeAssertion(nullness): Must not be null";
        assert publicPartB2 != null : "@AssumeAssertion(nullness): Must not be null";

        data2.copyOnWrite();
        publicPartA2.copyOnWrite();
        publicPartB2.copyOnWrite();

        ByteArrayOutputStream stream2 = new ByteArrayOutputStream();
        WritableByteChannel channel2 = Channels.newChannel(stream2);
        channel2.write(data2.createFlatBuffer().getByteBuffer().duplicate());

        TestTools.assertEqualAndNotNull(data.getAlias(), data2.getAlias());
        TestTools.assertEqualAndNotNull(publicPartA.getType(), publicPartA2.getType());
        TestTools.assertEqualAndNotNull(publicPartA.getSize(), publicPartA2.getSize());
        TestTools.assertEqualAndNotNull(publicPartA.getKey64(), publicPartA2.getKey64());
        TestTools.assertEqualAndNotNull(publicPartA.getKeyBytes(), publicPartA2.getKeyBytes());
        TestTools.assertEqualAndNotNull(publicPartB.getType(), publicPartB2.getType());
        TestTools.assertEqualAndNotNull(publicPartB.getSize(), publicPartB2.getSize());
        TestTools.assertEqualAndNotNull(publicPartB.getKey64(), publicPartB2.getKey64());
        TestTools.assertEqualAndNotNull(publicPartB.getKeyBytes(), publicPartB2.getKeyBytes());
        
        byte[] streamBytes1 = stream.toByteArray();
        byte[] streamBytes2 = stream2.toByteArray();

        ByteBuffer bb = ByteBuffer.wrap(streamBytes2);
        MessagePublicKeyDto data3 = new MessagePublicKeyDto(MessagePublicKey.getRootAsMessagePublicKey(bb));
        MessageKeyPartDto publicPartA3 = data3.getPublicParts().stream().findFirst().orElse(null);
        MessageKeyPartDto publicPartB3 = data3.getPublicParts().stream().skip(1).findFirst().orElse(null);
        assert publicPartA3 != null : "@AssumeAssertion(nullness): Must not be null";
        assert publicPartB2 != null : "@AssumeAssertion(nullness): Must not be null";

        data3.copyOnWrite();
        publicPartA3.copyOnWrite();
        publicPartB3.copyOnWrite();

        TestTools.assertEqualAndNotNull(data.getAlias(), data3.getAlias());
        TestTools.assertEqualAndNotNull(publicPartA.getType(), publicPartA3.getType());
        TestTools.assertEqualAndNotNull(publicPartA.getSize(), publicPartA3.getSize());
        TestTools.assertEqualAndNotNull(publicPartA.getKey64(), publicPartA3.getKey64());
        TestTools.assertEqualAndNotNull(publicPartA.getKeyBytes(), publicPartA3.getKeyBytes());
        TestTools.assertEqualAndNotNull(publicPartB.getType(), publicPartB3.getType());
        TestTools.assertEqualAndNotNull(publicPartB.getSize(), publicPartB3.getSize());
        TestTools.assertEqualAndNotNull(publicPartB.getKey64(), publicPartB3.getKey64());
        TestTools.assertEqualAndNotNull(publicPartB.getKeyBytes(), publicPartB3.getKeyBytes());

        TestTools.assertEqualAndNotNull(streamBytes1, streamBytes2);
    }
}
