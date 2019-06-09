package com.tokera.ate.test.msg;

import com.google.common.base.Objects;
import com.google.common.collect.Lists;
import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.dao.msg.MessageSecurityGate;
import com.tokera.ate.dto.msg.MessageSecurityCastleDto;
import com.tokera.ate.dto.msg.MessageSecurityGateDto;
import org.apache.commons.codec.binary.Base64;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.channels.Channels;
import java.nio.channels.WritableByteChannel;
import java.util.Random;
import java.util.UUID;

@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public class MessageSecurityCastleTests
{
    @Test
    public void forwardTestGate()
    {
        byte[] bytes1 = new byte[20];
        byte[] bytes2 = new byte[20];
        byte[] bytes3 = new byte[2000];
        new Random().nextBytes(bytes1);
        new Random().nextBytes(bytes2);
        new Random().nextBytes(bytes3);
        
        MessageSecurityGateDto data = new MessageSecurityGateDto(
                Base64.encodeBase64URLSafeString(bytes2),
                bytes3
        );

        MessageSecurityGateDto data2 = new MessageSecurityGateDto(data.createFlatBuffer());
        Assertions.assertArrayEquals(data.getEncryptedTextBytes(), data2.getEncryptedTextBytes());
        data2.setEncryptedTextBytes(bytes3);
        Assertions.assertTrue(Objects.equal(data.getPublicKeyHash(), data2.getPublicKeyHash()), "Public key hash is not equal");
        
        MessageSecurityGate base = data.createFlatBuffer();
        data2 = new MessageSecurityGateDto(base);
        Assertions.assertArrayEquals(data.getEncryptedTextBytes(), data2.getEncryptedTextBytes());
        data2.setEncryptedTextBytes(bytes3);
        Assertions.assertTrue(Objects.equal(data.getPublicKeyHash(), data2.getPublicKeyHash()), "Public key hash is not equal");
    }
    
    @Test
    public void serializeTestGates() throws IOException
    {
        byte[] bytes1 = new byte[20];
        byte[] bytes2 = new byte[20];
        byte[] bytes3 = new byte[2000];
        new Random().nextBytes(bytes1);
        new Random().nextBytes(bytes2);
        new Random().nextBytes(bytes3);

        MessageSecurityGateDto data = new MessageSecurityGateDto(
                Base64.encodeBase64URLSafeString(bytes2),
                bytes3
        );
        
        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        WritableByteChannel channel = Channels.newChannel(stream);
        channel.write(data.createFlatBuffer().getByteBuffer().duplicate());

        MessageSecurityGateDto data2 = new MessageSecurityGateDto(data.createFlatBuffer());
        
        ByteArrayOutputStream stream2 = new ByteArrayOutputStream();
        WritableByteChannel channel2 = Channels.newChannel(stream2);
        channel2.write(data2.createFlatBuffer().getByteBuffer().duplicate());
        
        Assertions.assertArrayEquals(data.getEncryptedTextBytes(), data2.getEncryptedTextBytes());
        Assertions.assertTrue(Objects.equal(data.getPublicKeyHash(), data2.getPublicKeyHash()), "Public key hash is not equal");
        
        byte[] streamBytes1 = stream.toByteArray();
        byte[] streamBytes2 = stream2.toByteArray();
        Assertions.assertArrayEquals(streamBytes1, streamBytes2);
    }
    @Test
    public void forwardTestCastle()
    {
        UUID id = UUID.randomUUID();
        byte[] bytes1 = new byte[20];
        byte[] bytes2 = new byte[20];
        byte[] bytes3 = new byte[2000];
        new Random().nextBytes(bytes1);
        new Random().nextBytes(bytes2);
        new Random().nextBytes(bytes3);

        MessageSecurityGateDto gate = new MessageSecurityGateDto(
                Base64.encodeBase64URLSafeString(bytes2),
                bytes3
        );

        MessageSecurityCastleDto data = new MessageSecurityCastleDto(
                id,
                Lists.newArrayList(gate)
        );

        MessageSecurityCastleDto data2 = new MessageSecurityCastleDto(data.createBaseFlatBuffer());
        Assertions.assertTrue(Objects.equal(data.getId(), data2.getId()));

        MessageBase base = data.createBaseFlatBuffer();
        data2 = new MessageSecurityCastleDto(base);
        Assertions.assertTrue(Objects.equal(data.getId(), data2.getId()));
    }

    @Test
    public void serializeTestCastle() throws IOException
    {
        UUID id = UUID.randomUUID();
        byte[] bytes1 = new byte[20];
        byte[] bytes2 = new byte[20];
        byte[] bytes3 = new byte[2000];
        new Random().nextBytes(bytes1);
        new Random().nextBytes(bytes2);
        new Random().nextBytes(bytes3);

        MessageSecurityGateDto gate = new MessageSecurityGateDto(
                Base64.encodeBase64URLSafeString(bytes2),
                bytes3
        );

        MessageSecurityCastleDto data = new MessageSecurityCastleDto(
                id,
                Lists.newArrayList(gate)
        );

        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        WritableByteChannel channel = Channels.newChannel(stream);
        channel.write(data.createBaseFlatBuffer().getByteBuffer().duplicate());

        MessageSecurityCastleDto data2 = new MessageSecurityCastleDto(data.createBaseFlatBuffer());

        ByteArrayOutputStream stream2 = new ByteArrayOutputStream();
        WritableByteChannel channel2 = Channels.newChannel(stream2);
        channel2.write(data2.createBaseFlatBuffer().getByteBuffer().duplicate());

        Assertions.assertTrue(Objects.equal(data.getId(), data2.getId()));

        byte[] streamBytes1 = stream.toByteArray();
        byte[] streamBytes2 = stream2.toByteArray();
        Assertions.assertArrayEquals(streamBytes1, streamBytes2);
    }
}
