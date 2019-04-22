package com.tokera.ate.test.msg;

import com.google.common.base.Objects;
import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.dao.kafka.MessageSerializer;
import com.tokera.ate.dao.msg.MessageDataHeader;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageDataHeaderDto;
import com.tokera.ate.test.dao.MyAccount;
import junit.framework.Assert;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.channels.Channels;
import java.nio.channels.WritableByteChannel;
import java.util.Set;
import java.util.UUID;

@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public class MessageDataHeaderTests
{
    
    @Test
    public void forwardTest()
    {
        UUID id = UUID.randomUUID();
        UUID version = UUID.randomUUID();
        UUID previousVersion = UUID.randomUUID();
        UUID merge1 = UUID.randomUUID();
        UUID merge2 = UUID.randomUUID();
        
        MessageDataHeaderDto header = new MessageDataHeaderDto(id, version, previousVersion, "Acount");
        header.setInheritRead(true);
        header.setInheritWrite(false);
        header.setEncryptKeyHash("HASHTEXT");
        header.getAllowRead().add("FIRSTKEY");
        header.getAllowRead().add("SECONDKEY");
        header.getMerges().add(merge1);
        header.getMerges().add(merge2);
        
        MessageDataHeaderDto header2 = new MessageDataHeaderDto(header.createFlatBuffer());
        Assert.assertTrue("ID is not equal", Objects.equal(header.getIdOrThrow(), header2.getIdOrThrow()));
        Assert.assertTrue("Version is not equal", Objects.equal(header.getVersionOrThrow(), header2.getVersionOrThrow()));
        Assert.assertTrue("Previous Version is not equal", Objects.equal(header.getPreviousVersion(), header2.getPreviousVersion()));
        header2.setId(id);
        Assert.assertTrue("Inherit read flag is not equal", Objects.equal(header.getInheritRead(), header2.getInheritRead()));
        Assert.assertTrue("Inherit write flag is not equal", Objects.equal(header.getInheritWrite(), header2.getInheritWrite()));
        Assert.assertTrue("Encrypt key hash is not equal", Objects.equal(header.getEncryptKeyHash(), header2.getEncryptKeyHash()));
        Assert.assertTrue("Payload class is not equal", Objects.equal(header.getPayloadClazzOrThrow(), header2.getPayloadClazzOrThrow()));
    }
    
    @Test
    public void forwardTest2() throws IOException
    {
        UUID id = UUID.randomUUID();
        UUID version = UUID.randomUUID();
        UUID previousVersion = UUID.randomUUID();
        UUID merge1 = UUID.randomUUID();
        UUID merge2 = UUID.randomUUID();
        
        MessageDataHeaderDto header = new MessageDataHeaderDto(id, version, previousVersion, MyAccount.class.getSimpleName());
        header.setInheritRead(true);
        header.setInheritWrite(false);
        header.setEncryptKeyHash("HASHTEXT");
        header.getAllowRead().add("FIRSTKEY");
        header.getAllowRead().add("SECONDKEY");
        header.getMerges().add(merge1);
        header.getMerges().add(merge2);
        
        MessageDataDto data = new MessageDataDto(header, null, null);

        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        WritableByteChannel channel = Channels.newChannel(stream);
        channel.write(data.createBaseFlatBuffer().getByteBuffer().duplicate());
        
        MessageBase msg = MessageBase.getRootAsMessageBase(ByteBuffer.wrap(stream.toByteArray()));
        data = new MessageDataDto(msg);
        
        MessageDataHeaderDto header2 = data.getHeader();
        Assert.assertTrue("ID is not equal", Objects.equal(header.getIdOrThrow(), header2.getIdOrThrow()));
        Assert.assertTrue("Version is not equal", Objects.equal(header.getVersionOrThrow(), header2.getVersionOrThrow()));
        header2.setId(id);
        Assert.assertTrue("Inherit read flag is not equal", Objects.equal(header.getInheritRead(), header2.getInheritRead()));
        Assert.assertTrue("Inherit write flag is not equal", Objects.equal(header.getInheritWrite(), header2.getInheritWrite()));
        Assert.assertTrue("Encrypt key hash is not equal", Objects.equal(header.getEncryptKeyHash(), header2.getEncryptKeyHash()));
        Assert.assertTrue("Payload class is not equal", Objects.equal(header.getPayloadClazzOrThrow(), header2.getPayloadClazzOrThrow()));
        Assert.assertTrue(header.getAllowRead().size() == header2.getAllowRead().size());
        Assert.assertTrue(header.getAllowWrite().size() == header2.getAllowWrite().size());
        Assert.assertTrue(header.getMerges().size() == header2.getMerges().size());
        for (String hash : header.getAllowRead()) {
            Assert.assertTrue(header2.getAllowRead().contains(hash));
        }
        for (String hash : header.getAllowWrite()) {
            Assert.assertTrue(header2.getAllowWrite().contains(hash));
        }
        for (UUID v : header.getMerges()) {
            Assert.assertTrue(header2.getMerges().contains(v));
        }
    }
    
    @Test
    public void streamTest() throws IOException
    {
        UUID id = UUID.randomUUID();
        UUID version = UUID.randomUUID();
        UUID previousVersion = UUID.randomUUID();
        UUID merge1 = UUID.randomUUID();
        UUID merge2 = UUID.randomUUID();
        
        MessageDataHeaderDto header = new MessageDataHeaderDto(id, version, previousVersion, MyAccount.class.getSimpleName());
        header.setInheritRead(true);
        header.setInheritWrite(false);
        header.setEncryptKeyHash("HASHTEXT123");
        header.getAllowRead().add("FIRSTKEY");
        header.getAllowRead().add("SECONDKEY");
        header.getMerges().add(merge1);
        header.getMerges().add(merge2);
        
        MessageDataHeader mdh = header.createFlatBuffer();
        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        WritableByteChannel channel = Channels.newChannel(stream);
        channel.write(mdh.getByteBuffer().duplicate());
        
        mdh = header.createFlatBuffer();
        MessageDataDto data = new MessageDataDto(new MessageDataHeaderDto(mdh), null, null);
        
        ByteArrayOutputStream stream2 = new ByteArrayOutputStream();
        WritableByteChannel channel2 = Channels.newChannel(stream2);
        channel2.write(data.getHeader().createFlatBuffer().getByteBuffer().duplicate());
        
        byte[] bytes1 = stream.toByteArray();
        byte[] bytes2 = stream2.toByteArray();
        Assertions.assertArrayEquals(bytes1, bytes2);
    }
    
    @Test
    public void serializeTest()
    {
        UUID version = UUID.randomUUID();
        UUID previousVersion = UUID.randomUUID();
        UUID merge1 = UUID.randomUUID();
        UUID merge2 = UUID.randomUUID();

        MessageDataHeaderDto header = new MessageDataHeaderDto(
                UUID.randomUUID(),
                version,
                previousVersion,
                MyAccount.class.getSimpleName()
        );
        header.getAllowWrite().add("AlxGQ-1JdtTPi7FWjG5PHPxQFssi4bjL-yis9zBBQvA");
        header.getMerges().add(merge1);
        header.getMerges().add(merge2);
        
        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        MessageSerializer.writeBytes(stream, header.createFlatBuffer());
        
        MessageDataDto data = new MessageDataDto(header, null, null);

        MessageDataDto data2 = new MessageDataDto(data.createFlatBuffer());
        MessageDataHeaderDto header2 = data2.getHeader();
        
        ByteArrayOutputStream stream2 = new ByteArrayOutputStream();
        MessageSerializer.writeBytes(stream2, data2.getHeader().createFlatBuffer());
        
        Assert.assertTrue("ID is not equal", Objects.equal(header.getIdOrThrow(), header2.getIdOrThrow()));
        Assert.assertTrue("Version is not equal", Objects.equal(header.getVersionOrThrow(), header2.getVersionOrThrow()));
        Assert.assertTrue("Inherit read flag is not equal", Objects.equal(header.getInheritRead(), header2.getInheritRead()));
        Assert.assertTrue("Inherit write flag is not equal", Objects.equal(header.getInheritWrite(), header2.getInheritWrite()));
        Assert.assertTrue("Encrypt key hash is not equal", Objects.equal(header.getEncryptKeyHash(), header2.getEncryptKeyHash()));
        Assert.assertTrue("Payload class is not equal", Objects.equal(header.getPayloadClazzOrThrow(), header2.getPayloadClazzOrThrow()));
        Assert.assertTrue(header.getAllowRead().size() == header2.getAllowRead().size());
        Assert.assertTrue(header.getAllowWrite().size() == header2.getAllowWrite().size());
        Assert.assertTrue(header.getMerges().size() == header2.getMerges().size());
        for (String hash : header.getAllowRead()) {
            Assert.assertTrue(header2.getAllowRead().contains(hash));
        }
        for (String hash : header.getAllowWrite()) {
            Assert.assertTrue(header2.getAllowWrite().contains(hash));
        }
        for (UUID v : header.getMerges()) {
            Set<UUID> header2parentVersions = header2.getMerges();
            Assert.assertTrue(header2parentVersions.contains(v));
        }
        
        byte[] bytes1 = stream.toByteArray();
        byte[] bytes2 = stream2.toByteArray();
        Assertions.assertArrayEquals(bytes1, bytes2);
    }
}
