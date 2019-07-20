/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto.msg;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.google.flatbuffers.FlatBufferBuilder;
import com.tokera.ate.common.CopyOnWrite;
import com.tokera.ate.common.Immutalizable;
import com.tokera.ate.dao.msg.*;
import com.tokera.ate.annotations.YamlTag;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import javax.validation.constraints.NotNull;
import javax.ws.rs.WebApplicationException;
import java.io.Serializable;
import java.nio.ByteBuffer;

/**
 * Represents a data message on the distributed commit log
 */
@Dependent
@YamlTag("msg.data")
public class MessageDataDto extends MessageBaseDto implements Serializable, CopyOnWrite, Immutalizable {

    private static final long serialVersionUID = -5267155098387197834L;

    @com.jsoniter.annotation.JsonIgnore
    @JsonIgnore
    private transient @Nullable MessageData fb;

    @JsonProperty
    @NotNull
    private MessageDataHeaderDto header;
    @JsonProperty
    @Nullable
    private MessageDataDigestDto digest;
    @JsonProperty
    private byte @Nullable [] payloadAsBytes;

    @com.jsoniter.annotation.JsonIgnore
    @JsonIgnore
    private transient boolean _immutable = false;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public MessageDataDto() {
    }

    public MessageDataDto(MessageDataHeaderDto header, @Nullable MessageDataDigestDto digest, byte @Nullable [] payload) {
        this.header = header;
        this.digest = digest;
        this.payloadAsBytes = payload;
    }
    
    public MessageDataDto(MessageData val) {
        fb = val;

        MessageDataHeader header = fb.header();
        MessageDataDigest digest = fb.digest();
        assert header != null : "@AssumeAssertion(nullness): MessageData will always have a valid header";

        this.header = new MessageDataHeaderDto(header);
        if (digest != null) {
            this.digest = new MessageDataDigestDto(digest);
        }
    }
    
    public MessageDataDto(MessageBase val) {
        if (val.msgType() == MessageType.MessageData)
        {
            MessageData table = new MessageData();
            val.msg(table);
            fb = table;

            MessageDataHeader header = table.header();
            MessageDataDigest digest = table.digest();
            assert header != null : "@AssumeAssertion(nullness): MessageData must always have a valid header";

            this.header = new MessageDataHeaderDto(header);

            if (digest != null) {
                this.digest = new MessageDataDigestDto(digest);
            }
        } else {
            throw new WebApplicationException("Invalidate message type [expected=MessageData, actual=" + val.msgType() + "]");
        }
    }

    @Override
    public void copyOnWrite() {
        MessageData lfb = fb;
        if (lfb == null) return;

        this.payloadAsBytes = null;
        if (lfb.payloadLength() > 0) {
            ByteBuffer bb = lfb.payloadAsByteBuffer();
            if (bb != null) {
                byte[] v = new byte[bb.remaining()];
                bb.get(v);
                this.payloadAsBytes = v;
            }
        }
        
        fb = null;
    }

    public @Nullable String getPayload() {
        copyOnWrite();
        byte[] bytes = payloadAsBytes;
        if (bytes == null) return null;
        return Base64.encodeBase64URLSafeString(bytes);
    }

    public void setPayload(String payload) {
        assert this._immutable == false;
        copyOnWrite();
        this.payloadAsBytes = Base64.decodeBase64(payload);
    }
    
    public boolean hasPayload() {
        copyOnWrite();
        return this.payloadAsBytes != null && this.payloadAsBytes.length > 0;
    }
    
    public byte @Nullable [] getPayloadBytes() {
        copyOnWrite();
        return this.payloadAsBytes;
    }
    
    public void setPayloadBytes(byte[] bytes) {
        assert this._immutable == false;
        copyOnWrite();
        this.payloadAsBytes = bytes;
    }

    public MessageDataHeaderDto getHeader() {
        return header;
    }

    public void setHeader(MessageDataHeaderDto header) {
        assert this._immutable == false;
        this.header = header;
    }
    
    public @Nullable MessageDataDigestDto getDigest() {
        return digest;
    }

    public void setDigest(MessageDataDigestDto digest) {
        assert this._immutable == false;
        this.digest = digest;
    }
    
    @Override
    public int flatBuffer(FlatBufferBuilder fbb)
    {
        int offsetHeader = this.getHeader().flatBuffer(fbb);

        int offsetDigest = -1;
        MessageDataDigestDto digest = this.getDigest();
        if (digest != null) {
            offsetDigest = digest.flatBuffer(fbb);
        }
        
        int offsetPayload = -1;
        byte[] payloadBytes = this.getPayloadBytes();
        if (payloadBytes != null) {
            offsetPayload = MessageData.createPayloadVector(fbb, payloadBytes);
        }
        
        MessageData.startMessageData(fbb);
        MessageData.addHeader(fbb, offsetHeader);
        if (offsetDigest >= 0) MessageData.addDigest(fbb, offsetDigest);
        if (offsetPayload >= 0) MessageData.addPayload(fbb, offsetPayload);
        return MessageData.endMessageData(fbb);
    }
    
    public MessageData createFlatBuffer()
    {
        FlatBufferBuilder fbb = new FlatBufferBuilder();
        fbb.finish(flatBuffer(fbb));
        return MessageData.getRootAsMessageData(fbb.dataBuffer());
    }

    public void immutalize() {
        if (this instanceof CopyOnWrite) {
            ((CopyOnWrite)this).copyOnWrite();
        }
        this._immutable = true;
        this.header.immutalize();
        if (this.digest != null) this.digest.immutalize();
    }
}