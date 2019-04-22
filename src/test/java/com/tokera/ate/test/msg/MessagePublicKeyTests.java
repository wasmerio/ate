package com.tokera.ate.test.msg;

import com.google.api.client.util.Base64;
import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.test.TestTools;
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
        
        MessagePublicKeyDto data = new MessagePublicKeyDto(bytes3);
        data.setAlias("THEALIAS");
        data.setPublicKeyHash(Base64.encodeBase64URLSafeString(bytes2));
        data.setPublicKey(Base64.encodeBase64URLSafeString(bytes3));
        
        MessagePublicKeyDto data2 = new MessagePublicKeyDto(data.createFlatBuffer());
        TestTools.assertEqualAndNotNull(data.getAlias(), data2.getAlias());
        TestTools.assertEqualAndNotNull(data.getPublicKey(), data2.getPublicKey());
        TestTools.assertEqualAndNotNull(data.getPublicKeyBytes(), data2.getPublicKeyBytes());
        data2.setAlias("THEALIAS");
        TestTools.assertEqualAndNotNull(data.getPublicKeyHash(), data2.getPublicKeyHash());
        TestTools.assertEqualAndNotNull(data.getPublicKey(), data2.getPublicKey());
        TestTools.assertEqualAndNotNull(data.getPublicKeyBytes(), data2.getPublicKeyBytes());
        
        MessageBase base = data.createBaseFlatBuffer();
        data2 = new MessagePublicKeyDto(base);
        TestTools.assertEqualAndNotNull(data.getAlias(), data2.getAlias());
        TestTools.assertEqualAndNotNull(data.getPublicKey(), data2.getPublicKey());
        TestTools.assertEqualAndNotNull(data.getPublicKeyBytes(), data2.getPublicKeyBytes());
        data2.setAlias("THEALIAS");
        TestTools.assertEqualAndNotNull(data.getPublicKeyHash(), data2.getPublicKeyHash());
        TestTools.assertEqualAndNotNull(data.getPublicKey(), data2.getPublicKey());
        TestTools.assertEqualAndNotNull(data.getPublicKeyBytes(), data2.getPublicKeyBytes());
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
        
        MessagePublicKeyDto data = new MessagePublicKeyDto(bytes3);
        data.setAlias("THEALIAS");
        data.setPublicKeyHash(Base64.encodeBase64URLSafeString(bytes2));
        data.setPublicKey(Base64.encodeBase64URLSafeString(bytes3));
        
        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        WritableByteChannel channel = Channels.newChannel(stream);
        channel.write(data.createFlatBuffer().getByteBuffer().duplicate());
        
        MessagePublicKeyDto data2 = new MessagePublicKeyDto(data.createFlatBuffer());
        
        ByteArrayOutputStream stream2 = new ByteArrayOutputStream();
        WritableByteChannel channel2 = Channels.newChannel(stream2);
        channel2.write(data2.createFlatBuffer().getByteBuffer().duplicate());

        TestTools.assertEqualAndNotNull(data.getAlias(), data2.getAlias());
        TestTools.assertEqualAndNotNull(data.getPublicKey(), data2.getPublicKey());
        TestTools.assertEqualAndNotNull(data.getPublicKeyBytes(), data2.getPublicKeyBytes());
        
        byte[] streamBytes1 = stream.toByteArray();
        byte[] streamBytes2 = stream2.toByteArray();
        TestTools.assertEqualAndNotNull(streamBytes1, streamBytes2);
    }
}
