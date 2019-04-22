package com.tokera.ate.test.msg;

import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.dto.msg.MessageSyncDto;
import junit.framework.Assert;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.channels.Channels;
import java.nio.channels.WritableByteChannel;
import java.util.Random;

@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public class MessageSyncTests
{
    @Test
    public void forwardTest()
    {
        MessageSyncDto data = new MessageSyncDto(
                new Random().nextLong(),
                new Random().nextLong()
        );

        MessageSyncDto data2 = new MessageSyncDto(data.createFlatBuffer());
        Assert.assertEquals(data.getTicket1(), data2.getTicket1());
        Assert.assertEquals(data.getTicket2(), data2.getTicket2());

        MessageBase base = data.createBaseFlatBuffer();
        data2 = new MessageSyncDto(base);
        Assert.assertEquals(data.getTicket1(), data2.getTicket1());
        Assert.assertEquals(data.getTicket2(), data2.getTicket2());
    }
    
    @Test
    public void serializeTest() throws IOException
    {
        MessageSyncDto data = new MessageSyncDto(
                new Random().nextLong(),
                new Random().nextLong()
        );

        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        WritableByteChannel channel = Channels.newChannel(stream);
        channel.write(data.createFlatBuffer().getByteBuffer().duplicate());
        
        MessageSyncDto data2 = new MessageSyncDto(data.createFlatBuffer());
        
        ByteArrayOutputStream stream2 = new ByteArrayOutputStream();
        WritableByteChannel channel2 = Channels.newChannel(stream2);
        channel2.write(data2.createFlatBuffer().getByteBuffer().duplicate());

        Assert.assertEquals(data.getTicket1(), data2.getTicket1());
        Assert.assertEquals(data.getTicket2(), data2.getTicket2());
    }
}
