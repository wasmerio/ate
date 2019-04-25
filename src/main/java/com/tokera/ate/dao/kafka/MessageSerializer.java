package com.tokera.ate.dao.kafka;

import com.tokera.ate.dao.msg.*;
import com.tokera.ate.dto.msg.*;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.channels.Channels;
import java.nio.channels.WritableByteChannel;
import java.security.MessageDigest;
import java.util.Map;
import java.util.UUID;

import com.tokera.ate.units.Hash;
import org.apache.commons.codec.binary.Base64;
import org.apache.kafka.common.serialization.Serializer;
import org.slf4j.LoggerFactory;

/**
 * Kafka serializer used for the main message flatbuffers
 */
public class MessageSerializer implements Serializer<MessageBase> {
    
    private static final org.slf4j.Logger SLOG = LoggerFactory.getLogger(MessageSerializer.class);
    
    private static MessageDigest g_sha256digest;
    
    static {
        try {
            g_sha256digest = MessageDigest.getInstance("SHA-256");
        } catch (Exception ex) {
            throw new RuntimeException(ex);
        }
    }
    
    @Override
    public void configure(Map<String, ?> map, boolean bln) {
    }

    @Override
    public byte[] serialize(String topic, MessageBase obj) {
        ByteBuffer bb = obj.getByteBuffer().duplicate();
        byte[] ret = new byte[bb.remaining()];
        bb.get(ret);
        return ret;
    }

    @Override
    public void close() {
    }
    
    static public void writeBytes(ByteArrayOutputStream stream, MessageDataHeader header)
    {
        try {
            WritableByteChannel channel = Channels.newChannel(stream);
            channel.write(header.getByteBuffer().duplicate());
        } catch (IOException ex) {
            throw new RuntimeException(ex);
        }
    }
    
    public static String getKey(MessageBaseDto msg)
    {
        if (msg instanceof MessageDataDto) {
            return getKey((MessageDataDto)msg);
        }
        if (msg instanceof MessageEncryptTextDto) {
            return getKey((MessageEncryptTextDto)msg);
        }
        if (msg instanceof MessagePublicKeyDto) {
            return getKey((MessagePublicKeyDto)msg);
        }
        if (msg instanceof MessageSyncDto) {
            return getKey((MessageSyncDto)msg);
        }
        throw new RuntimeException("Unable to generate key for message [type=" + msg.getClass() + "]");
    }
    
    public static String getKey(MessageBase msg)
    {
        switch (msg.msgType())
        {
            case MessageType.MessageData:
            {
                MessageData data = (MessageData)msg.msg(new MessageData());
                if (data != null) return getKey(data);
            }
            case MessageType.MessageEncryptText:
            {
                MessageEncryptText text = (MessageEncryptText)msg.msg(new MessageEncryptText());
                if (text != null) return getKey(text);
            }
            case MessageType.MessagePublicKey:
            {
                MessagePublicKey key = (MessagePublicKey)msg.msg(new MessagePublicKey());
                if (key != null) return getKey(key);
            }
            case MessageType.MessageSync:
            {
                MessageSync sync = (MessageSync)msg.msg(new MessageSync());
                if (sync != null) return getKey(sync);
            }
        }
        throw new RuntimeException("Unable to generate key for message [type=" + msg.getClass() + "]");
    }
    
    public static String getKey(MessageData data)
    {
        MessageDataHeader header = data.header();
        if (header == null) throw new RuntimeException("MessageData does not have a header");
        return new UUID(header.id().high(), header.id().low()).toString();
    }

    public static String getKey(MessageSync sync) { return "sync:" + sync.ticket1() + ":" + sync.ticket2(); }
    
    public static String getKey(MessageEncryptText text)
    {
        return text.publicKeyHash() + ":" + text.textHash();
    }
    
    public static String getKey(MessagePublicKey key)
    {
        @Hash String ret = key.publicKeyHash();
        if (ret == null) throw new RuntimeException("MessagePublicKey does not have a hash.");
        return ret;
    }
    
    public static String getKey(MessageDataDto data)
    {
        StringBuilder sb = new StringBuilder();
        sb.append(data.getHeader().getIdOrThrow().toString());
        sb.append(",");

        MessageDataDigestDto dataDigest = data.getDigest();
        if (dataDigest != null) {
            sb.append(dataDigest.getPublicKeyHash());
        }

        for (String child : data.getHeader().getAllowWrite()) {
            sb.append(",");
            sb.append(child);
        }
        
        try {
            MessageDigest digest = (MessageDigest)g_sha256digest.clone();
            byte[] digestBytes = digest.digest(sb.toString().getBytes());
            return Base64.encodeBase64URLSafeString(digestBytes);
        } catch (CloneNotSupportedException ex) {
            String msg = ex.getMessage();
            if (msg == null) msg = ex.getClass().getSimpleName();
            SLOG.warn(msg, ex);
            return sb.toString();
        }
    }
    
    public static String getKey(MessageEncryptTextDto text)
    {
        return text.getPublicKeyHash() + ":" + text.getTextHash();
    }
    
    public static String getKey(MessagePublicKeyDto key)
    {
        @Hash String hash = key.getPublicKeyHash();
        if (hash == null) throw new RuntimeException("Public key has no hash");
        return hash;
    }

    public static String getKey(MessageSyncDto key)
    {
        return "sync:" + key.getTicket1() + ":" + key.getTicket2();
    }
}