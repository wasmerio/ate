package com.tokera.ate.test.msg;

import com.google.common.base.Objects;
import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.dto.msg.MessageEncryptTextDto;
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
public class MessageEncryptTextTests
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
        
        MessageEncryptTextDto data = new MessageEncryptTextDto(
                Base64.encodeBase64URLSafeString(bytes2),
                Base64.encodeBase64URLSafeString(bytes1),
                bytes3
        );
        
        MessageEncryptTextDto data2 = new MessageEncryptTextDto(data.createFlatBuffer());
        Assertions.assertArrayEquals(data.getEncryptedTextBytes(), data2.getEncryptedTextBytes());
        data2.setEncryptedTextBytes(bytes3);
        Assertions.assertTrue(Objects.equal(data.getTextHash(), data2.getTextHash()), "Text hash is not equal");
        Assertions.assertTrue(Objects.equal(data.getPublicKeyHash(), data2.getPublicKeyHash()), "Public key hash is not equal");
        
        MessageBase base = data.createBaseFlatBuffer();
        data2 = new MessageEncryptTextDto(base);
        Assertions.assertArrayEquals(data.getEncryptedTextBytes(), data2.getEncryptedTextBytes());
        data2.setEncryptedTextBytes(bytes3);
        Assertions.assertTrue(Objects.equal(data.getTextHash(), data2.getTextHash()), "Text hash is not equal");
        Assertions.assertTrue(Objects.equal(data.getPublicKeyHash(), data2.getPublicKeyHash()), "Public key hash is not equal");
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
        
        MessageEncryptTextDto data = new MessageEncryptTextDto(
                Base64.encodeBase64URLSafeString(bytes2),
                Base64.encodeBase64URLSafeString(bytes1),
                bytes3
        );
        
        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        WritableByteChannel channel = Channels.newChannel(stream);
        channel.write(data.createFlatBuffer().getByteBuffer().duplicate());
        
        MessageEncryptTextDto data2 = new MessageEncryptTextDto(data.createFlatBuffer());
        
        ByteArrayOutputStream stream2 = new ByteArrayOutputStream();
        WritableByteChannel channel2 = Channels.newChannel(stream2);
        channel2.write(data2.createFlatBuffer().getByteBuffer().duplicate());
        
        Assertions.assertArrayEquals(data.getEncryptedTextBytes(), data2.getEncryptedTextBytes());
        Assertions.assertTrue(Objects.equal(data.getTextHash(), data2.getTextHash()), "Text hash is not equal");
        Assertions.assertTrue(Objects.equal(data.getPublicKeyHash(), data2.getPublicKeyHash()), "Public key hash is not equal");
        
        byte[] streamBytes1 = stream.toByteArray();
        byte[] streamBytes2 = stream2.toByteArray();
        Assertions.assertArrayEquals(streamBytes1, streamBytes2);
    }
}
