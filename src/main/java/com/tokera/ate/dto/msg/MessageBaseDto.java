package com.tokera.ate.dto.msg;

import com.google.flatbuffers.FlatBufferBuilder;
import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.dao.msg.MessageType;
import com.tokera.ate.delegates.AteDelegate;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.ws.rs.WebApplicationException;
import java.io.Serializable;

/**
 * Base message on the Kafka stream that wraps a flatbuffer
 */
public abstract class MessageBaseDto implements Serializable
{
    private static final long serialVersionUID = -5384759189505057786L;

    public abstract int flatBuffer(FlatBufferBuilder fbb);

    public static @Nullable MessageBaseDto from(MessageBase raw) {
        MessageBaseDto msg;
        switch (raw.msgType()) {
            case MessageType.MessageData:
                msg = new MessageDataDto(raw);
                break;
            case MessageType.MessageSecurityCastle:
                msg = new MessageSecurityCastleDto(raw);
                break;
            case MessageType.MessagePublicKey:
                msg = new MessagePublicKeyDto(raw);
                break;
            case MessageType.MessageSync:
                msg = new MessageSyncDto(raw);
                break;
            default:
                AteDelegate.get().genericLogger.warn("unknown message type: " + raw.msgType());
                return null;
        }
        return msg;
    }
    
    public MessageBase createBaseFlatBuffer()
    {
        FlatBufferBuilder fbb = new FlatBufferBuilder();

        int offsetData = flatBuffer(fbb);

        MessageBase.startMessageBase(fbb);
        if (this instanceof MessageDataDto) {
            MessageBase.addMsgType(fbb, MessageType.MessageData);
        } else if (this instanceof MessageSecurityCastleDto) {
            MessageBase.addMsgType(fbb, MessageType.MessageSecurityCastle);
        } else if (this instanceof MessagePublicKeyDto) {
            MessageBase.addMsgType(fbb, MessageType.MessagePublicKey);
        } else if (this instanceof MessageSyncDto) {
            MessageBase.addMsgType(fbb, MessageType.MessageSync);
        } else {
            throw new WebApplicationException("Unsupported message type [clazz=" + this.getClass() + "]");
        }
        MessageBase.addMsg(fbb, offsetData);
        fbb.finish(MessageBase.endMessageBase(fbb));

        return MessageBase.getRootAsMessageBase(fbb.dataBuffer());
    }
}
