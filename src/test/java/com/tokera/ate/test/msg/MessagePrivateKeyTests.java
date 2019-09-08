package com.tokera.ate.test.msg;

import com.google.common.collect.Lists;
import com.tokera.ate.dao.enumerations.KeyType;
import com.tokera.ate.dto.msg.MessageKeyPartDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.client.TestTools;
import java.util.Collections;
import org.apache.commons.codec.binary.Base64;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.channels.Channels;
import java.nio.channels.WritableByteChannel;
import java.util.Random;

@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public class MessagePrivateKeyTests
{

    @Test
    public void forwardTest()
    {
        byte[] bytes1 = new byte[20];
        byte[] bytes2 = new byte[20];
        byte[] bytes3 = new byte[2000];
        byte[] bytes4 = new byte[20];
        byte[] bytes5 = new byte[2000];
        new Random().nextBytes(bytes1);
        new Random().nextBytes(bytes2);
        new Random().nextBytes(bytes3);
        new Random().nextBytes(bytes4);
        new Random().nextBytes(bytes5);

        MessageKeyPartDto publicPart = new MessageKeyPartDto(KeyType.ntru, 128, Base64.encodeBase64URLSafeString(bytes3));
        MessageKeyPartDto privatePart = new MessageKeyPartDto(KeyType.ntru, 128, Base64.encodeBase64URLSafeString(bytes5));
        MessagePrivateKeyDto data = new MessagePrivateKeyDto(Collections.singletonList(publicPart), Lists.newArrayList(privatePart));
        data.setAlias("THEALIAS");
        data.setPublicKeyHash(Base64.encodeBase64URLSafeString(bytes2));
        data.setPrivateKeyHash(Base64.encodeBase64URLSafeString(bytes4));

        MessagePrivateKeyDto data2 = new MessagePrivateKeyDto(data.createPrivateKeyFlatBuffer());
        MessageKeyPartDto publicPart2 = data2.getPublicParts().stream().findFirst().orElse(null);
        MessageKeyPartDto privatePart2 = data2.getPrivateParts().stream().findFirst().orElse(null);
        assert publicPart2 != null : "@AssumeAssertion(nullness): Must not be null";
        assert privatePart2 != null : "@AssumeAssertion(nullness): Must not be null";

        Assertions.assertTrue(publicPart2.getKeyBytes() != null);
        Assertions.assertTrue(privatePart2.getKeyBytes() != null);

        TestTools.assertEqualAndNotNull(data.getAlias(), data2.getAlias());
        TestTools.assertEqualAndNotNull(publicPart.getKey64(), publicPart2.getKey64());
        TestTools.assertEqualAndNotNull(publicPart.getKeyBytes(), publicPart2.getKeyBytes());
        TestTools.assertEqualAndNotNull(data.getPublicKeyHash(), data2.getPublicKeyHash());
        TestTools.assertEqualAndNotNull(privatePart.getKey64(), privatePart2.getKey64());
        TestTools.assertEqualAndNotNull(privatePart.getKeyBytes(), privatePart2.getKeyBytes());
        TestTools.assertEqualAndNotNull(data.getPrivateKeyHash(), data2.getPrivateKeyHash());

        data2.setAlias("THEALIAS");

        TestTools.assertEqualAndNotNull(data.getAlias(), data2.getAlias());
        TestTools.assertEqualAndNotNull(publicPart.getKey64(), publicPart2.getKey64());
        TestTools.assertEqualAndNotNull(publicPart.getKeyBytes(), publicPart2.getKeyBytes());
        TestTools.assertEqualAndNotNull(data.getPublicKeyHash(), data2.getPublicKeyHash());
        TestTools.assertEqualAndNotNull(privatePart.getKey64(), privatePart2.getKey64());
        TestTools.assertEqualAndNotNull(privatePart.getKeyBytes(), privatePart2.getKeyBytes());
        TestTools.assertEqualAndNotNull(data.getPrivateKeyHash(), data2.getPrivateKeyHash());
    }

    @Test
    public void serializeTest() throws IOException
    {
        byte[] bytes1 = new byte[20];
        byte[] bytes2 = new byte[20];
        byte[] bytes3 = new byte[2000];
        byte[] bytes4 = new byte[20];
        byte[] bytes5 = new byte[2000];
        new Random().nextBytes(bytes1);
        new Random().nextBytes(bytes2);
        new Random().nextBytes(bytes3);
        new Random().nextBytes(bytes4);
        new Random().nextBytes(bytes5);

        MessageKeyPartDto publicPart = new MessageKeyPartDto(KeyType.ntru, 128, Base64.encodeBase64URLSafeString(bytes3));
        MessageKeyPartDto privatePart = new MessageKeyPartDto(KeyType.ntru, 128, Base64.encodeBase64URLSafeString(bytes5));
        MessagePrivateKeyDto data = new MessagePrivateKeyDto(Collections.singletonList(publicPart), Lists.newArrayList(privatePart));
        data.setAlias("THEALIAS");
        data.setPublicKeyHash(Base64.encodeBase64URLSafeString(bytes2));
        data.setPrivateKeyHash(Base64.encodeBase64URLSafeString(bytes4));

        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        WritableByteChannel channel = Channels.newChannel(stream);
        channel.write(data.createPrivateKeyFlatBuffer().getByteBuffer().duplicate());

        MessagePrivateKeyDto data2 = new MessagePrivateKeyDto(data.createPrivateKeyFlatBuffer());

        ByteArrayOutputStream stream2 = new ByteArrayOutputStream();
        WritableByteChannel channel2 = Channels.newChannel(stream2);
        channel2.write(data2.createPrivateKeyFlatBuffer().getByteBuffer().duplicate());

        MessageKeyPartDto publicPart2 = data2.getPublicParts().stream().findFirst().orElse(null);
        MessageKeyPartDto privatePart2 = data2.getPrivateParts().stream().findFirst().orElse(null);
        assert publicPart2 != null : "@AssumeAssertion(nullness): Must not be null";
        assert privatePart2 != null : "@AssumeAssertion(nullness): Must not be null";

        Assertions.assertTrue(publicPart2.getKeyBytes() != null);
        Assertions.assertTrue(privatePart2.getKeyBytes() != null);

        TestTools.assertEqualAndNotNull(data.getAlias(), data2.getAlias());
        TestTools.assertEqualAndNotNull(publicPart.getKey64(), publicPart2.getKey64());
        TestTools.assertEqualAndNotNull(publicPart.getKeyBytes(), publicPart2.getKeyBytes());
        TestTools.assertEqualAndNotNull(data.getPublicKeyHash(), data2.getPublicKeyHash());
        TestTools.assertEqualAndNotNull(privatePart.getKey64(), privatePart2.getKey64());
        TestTools.assertEqualAndNotNull(privatePart.getKeyBytes(), privatePart2.getKeyBytes());
        TestTools.assertEqualAndNotNull(data.getPrivateKeyHash(), data2.getPrivateKeyHash());

        byte[] streamBytes1 = stream.toByteArray();
        byte[] streamBytes2 = stream2.toByteArray();
        TestTools.assertEqualAndNotNull(streamBytes1, streamBytes2);
    }
}
