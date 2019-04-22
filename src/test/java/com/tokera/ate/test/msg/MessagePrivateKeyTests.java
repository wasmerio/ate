package com.tokera.ate.test.msg;

import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.test.TestTools;
import org.apache.commons.codec.binary.Base64;
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
        
        MessagePrivateKeyDto data = new MessagePrivateKeyDto(bytes3, bytes5);
        data.setAlias("THEALIAS");
        data.setPublicKeyHash(Base64.encodeBase64URLSafeString(bytes2));
        data.setPublicKey(Base64.encodeBase64URLSafeString(bytes3));
        data.setPrivateKeyHash(Base64.encodeBase64URLSafeString(bytes4));
        data.setPrivateKey(Base64.encodeBase64URLSafeString(bytes5));
        
        MessagePrivateKeyDto data2 = new MessagePrivateKeyDto(data.createPrivateKeyFlatBuffer());
        TestTools.assertEqualAndNotNull(data.getAlias(), data2.getAlias());
        TestTools.assertEqualAndNotNull(data.getPublicKey(), data2.getPublicKey());
        TestTools.assertEqualAndNotNull(data.getPublicKeyBytes(), data2.getPublicKeyBytes());
        TestTools.assertEqualAndNotNull(data.getPublicKeyHash(), data2.getPublicKeyHash());
        TestTools.assertEqualAndNotNull(data.getPrivateKey(), data2.getPrivateKey());
        TestTools.assertEqualAndNotNull(data.getPrivateKeyBytes(), data2.getPrivateKeyBytes());
        TestTools.assertEqualAndNotNull(data.getPrivateKeyHash(), data2.getPrivateKeyHash());
        data2.setAlias("THEALIAS");
        TestTools.assertEqualAndNotNull(data.getAlias(), data2.getAlias());
        TestTools.assertEqualAndNotNull(data.getPublicKey(), data2.getPublicKey());
        TestTools.assertEqualAndNotNull(data.getPublicKeyBytes(), data2.getPublicKeyBytes());
        TestTools.assertEqualAndNotNull(data.getPublicKeyHash(), data2.getPublicKeyHash());
        TestTools.assertEqualAndNotNull(data.getPrivateKey(), data2.getPrivateKey());
        TestTools.assertEqualAndNotNull(data.getPrivateKeyBytes(), data2.getPrivateKeyBytes());
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
        
        MessagePrivateKeyDto data = new MessagePrivateKeyDto(bytes3, bytes5);
        data.setAlias("THEALIAS");
        data.setPublicKeyHash(Base64.encodeBase64URLSafeString(bytes2));
        data.setPublicKey(Base64.encodeBase64URLSafeString(bytes3));
        data.setPrivateKeyHash(Base64.encodeBase64URLSafeString(bytes4));
        data.setPrivateKey(Base64.encodeBase64URLSafeString(bytes5));
        
        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        WritableByteChannel channel = Channels.newChannel(stream);
        channel.write(data.createPrivateKeyFlatBuffer().getByteBuffer().duplicate());
        
        MessagePrivateKeyDto data2 = new MessagePrivateKeyDto(data.createPrivateKeyFlatBuffer());
        
        ByteArrayOutputStream stream2 = new ByteArrayOutputStream();
        WritableByteChannel channel2 = Channels.newChannel(stream2);
        channel2.write(data2.createPrivateKeyFlatBuffer().getByteBuffer().duplicate());

        TestTools.assertEqualAndNotNull(data.getAlias(), data2.getAlias());
        TestTools.assertEqualAndNotNull(data.getPublicKey(), data2.getPublicKey());
        TestTools.assertEqualAndNotNull(data.getPublicKeyBytes(), data2.getPublicKeyBytes());
        TestTools.assertEqualAndNotNull(data.getPublicKeyHash(), data2.getPublicKeyHash());
        TestTools.assertEqualAndNotNull(data.getPrivateKey(), data2.getPrivateKey());
        TestTools.assertEqualAndNotNull(data.getPrivateKeyBytes(), data2.getPrivateKeyBytes());
        TestTools.assertEqualAndNotNull(data.getPrivateKeyHash(), data2.getPrivateKeyHash());
        
        byte[] streamBytes1 = stream.toByteArray();
        byte[] streamBytes2 = stream2.toByteArray();
        TestTools.assertEqualAndNotNull(streamBytes1, streamBytes2);
    }
}
