/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto.msg;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.google.flatbuffers.FlatBufferBuilder;
import com.google.gson.annotations.Expose;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.dao.msg.*;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.validation.constraints.NotNull;
import javax.ws.rs.WebApplicationException;
import java.io.Serializable;
import java.util.*;

/**
 * Represents a synchronization point for a bunch of data that was pushed onto the BUS
 */
@YamlTag("msg.sync")
public class MessageSyncDto extends MessageBaseDto implements Serializable {

    private static final long serialVersionUID = -8152777200711190736L;

    // When running in copy-on-write mode
    @Nullable
    private transient MessageSync fb;

    // Fields that are serialized
    @Expose
    @JsonProperty
    @NotNull
    private long ticket1;                   // Ticket ID that we will be waiting for
    @Expose
    @JsonProperty
    @NotNull
    private long ticket2;                   // Ticket ID that we will be waiting for

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public MessageSyncDto()
    {
        ticket1 = 0L;
        ticket2 = 0L;
    }

    public MessageSyncDto(long ticket1, long ticket2) {
        this.ticket1 = ticket1;
        this.ticket2 = ticket2;
    }

    public MessageSyncDto(MessageSync val)
    {
        this.fb = val;
    }

    public MessageSyncDto(MessageBase val) {
        if (val.msgType() == MessageType.MessageSync) {
            fb = (MessageSync)val.msg(new MessageSync());
        } else {
            throw new WebApplicationException("Invalidate message type [expected=MessageSync, actual=" + val.msgType() + "]");
        }
    }
    
    public void setFlatBuffer(MessageSync val) {
        this.fb = val;
    }
    
    public void copyOnWrite()
    {
        MessageSync lfb = fb;
        if (lfb == null) return;

        ticket1 = lfb.ticket1();
        ticket2 = lfb.ticket2();
        
        fb = null;
    }
    
    public long getTicket1() {
        MessageSync lfb = fb;
        if (lfb != null) {
            return lfb.ticket1();
        }
        return ticket1;
    }

    public long getTicket2() {
        MessageSync lfb = fb;
        if (lfb != null) {
            return lfb.ticket2();
        }
        return ticket2;
    }

    public void setTicket1(long val) {
        copyOnWrite();
        this.ticket1 = val;
    }

    public void setTicket2(long val) {
        copyOnWrite();
        this.ticket2 = val;
    }

    @Override
    public int flatBuffer(FlatBufferBuilder fbb)
    {
        MessageSync.startMessageSync(fbb);
        MessageSync.addTicket1(fbb, this.getTicket1());
        MessageSync.addTicket2(fbb, this.getTicket2());
        return MessageSync.endMessageSync(fbb);
    }
    
    public MessageSync createFlatBuffer()
    {
        FlatBufferBuilder fbb = new FlatBufferBuilder();
        fbb.finish(flatBuffer(fbb));
        return MessageSync.getRootAsMessageSync(fbb.dataBuffer());
    }

    @Override
    public boolean equals(@Nullable Object o)
    {
        if (o == null) return false;
        if (getClass() != o.getClass()) return false;

        MessageSyncDto that = (MessageSyncDto) o;

        if (Objects.equals(this.ticket1, that.ticket1) == false) return false;
        if (Objects.equals(this.ticket2, that.ticket2) == false) return false;

        return true;
    }

    @Override
    public int hashCode()
    {
        int result = (int)0;
        result += ((Long)this.ticket1).hashCode();
        result += ((Long)this.ticket2).hashCode();
        return result;
    }
}
