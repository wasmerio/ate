package com.tokera.ate.dto.msg;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.google.common.collect.Iterables;
import com.google.flatbuffers.FlatBufferBuilder;
import com.tokera.ate.common.*;
import com.tokera.ate.dao.ObjId;
import com.tokera.ate.dao.msg.*;
import com.tokera.ate.units.DaoId;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.ws.rs.WebApplicationException;
import java.io.Serializable;
import java.util.*;

public class MessageSecurityCastleDto extends MessageBaseDto implements Serializable, CopyOnWrite, Immutalizable {
    private static final long serialVersionUID = 1352819172417741228L;

    private transient @Nullable MessageSecurityCastle fb;
    protected transient @Nullable Integer hashCache = null;
    protected transient Map<String, String> lookupCache = null;

    @JsonProperty
    @MonotonicNonNull
    private @DaoId UUID id;
    @JsonProperty
    private ImmutalizableArrayList<MessageSecurityGateDto> gates = new ImmutalizableArrayList<>();

    @JsonIgnore
    private transient boolean _immutable = false;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public MessageSecurityCastleDto(){
    }

    public MessageSecurityCastleDto(@DaoId UUID id) {
        this.id = id;
    }

    public MessageSecurityCastleDto(@DaoId UUID id, Iterable<MessageSecurityGateDto> gates) {
        this.id = id;
        gates.forEach(g -> this.gates.add(g));
    }

    public MessageSecurityCastleDto(MessageSecurityCastle val) {
        fb = val;
    }

    public MessageSecurityCastleDto(MessageBase val) {
        if (val.msgType() == MessageType.MessageSecurityCastle) {
            fb = (MessageSecurityCastle)val.msg(new MessageSecurityCastle());
        } else {
            throw new WebApplicationException("Invalidate message type [expected=MessageSecurityCastle, actual=" + val.msgType() + "]");
        }
    }

    public void setFlatBuffer(MessageSecurityCastle val) {
        assert this._immutable == false;
        this.fb = val;
    }

    @Override
    public void copyOnWrite()
    {
        this.hashCache = null;
        this.lookupCache = null;

        MessageSecurityCastle lfb = fb;
        if (lfb == null) return;

        id = UUIDTools.convertUUID(lfb.id());

        this.gates.clear();
        if (lfb.gatesLength() > 0) {
            for (int n = 0; n < lfb.gatesLength(); n++) {
                MessageSecurityGate gate = lfb.gates(n);
                this.gates.add(new MessageSecurityGateDto(gate));
            }
        }

        fb = null;
    }

    public @Nullable @DaoId UUID getId() {
        MessageSecurityCastle lfb = fb;
        if (lfb != null) {
            return UUIDTools.convertUUID(lfb.id());
        }
        return this.id;
    }

    public @DaoId UUID getIdOrThrow() {
        MessageSecurityCastle lfb = fb;
        if (lfb != null) {
            return UUIDTools.convertUUID(lfb.id());
        }
        @DaoId UUID ret = this.id;
        if (ret == null) throw new WebApplicationException("Message security castle has no ID attached");
        return ret;
    }

    public void setId(@DaoId UUID id) {
        assert this._immutable == false;
        copyOnWrite();
        this.id = id;
    }

    public ImmutalizableArrayList<MessageSecurityGateDto> getGates() {
        MessageSecurityCastle lfb = fb;
        if (lfb != null) {
            ImmutalizableArrayList<MessageSecurityGateDto> ret = new ImmutalizableArrayList<>();
            ret.clear();
            if (lfb.gatesLength() > 0) {
                for (int n = 0; n < lfb.gatesLength(); n++) {
                    MessageSecurityGate gate = lfb.gates(n);
                    ret.add(new MessageSecurityGateDto(gate));
                }
            }
            return ret;
        }

        return this.gates;
    }

    public void setGates(ImmutalizableArrayList<MessageSecurityGateDto> gates) {
        copyOnWrite();
        this.lookupCache = null;
        this.hashCache = null;
        this.gates = gates;
    }

    @Override
    public int flatBuffer(FlatBufferBuilder fbb) {
        Iterable<MessageSecurityGateDto> gates = this.getGates();
        int offsetGates = -1;
        if (gates != null) {
            int size = Iterables.size(gates);
            int[] partOffsets = new int[size];

            int n = 0;
            for (MessageSecurityGateDto part : gates) {
                partOffsets[n++] = part.flatBuffer(fbb);
            }

            offsetGates = MessagePublicKey.createPartsVector(fbb, partOffsets);
        }

        MessageSecurityCastle.startMessageSecurityCastle(fbb);
        MessageSecurityCastle.addId(fbb, ObjId.createObjId(fbb, this.getIdOrThrow().getLeastSignificantBits(), this.getIdOrThrow().getMostSignificantBits()));
        if (offsetGates > 0) {
            MessageSecurityCastle.addGates(fbb, offsetGates);
        }
        return MessageSecurityCastle.endMessageSecurityCastle(fbb);
    }

    @Override
    public void immutalize() {
        this._immutable = true;
        this.gates.immutalize();
    }

    @Override
    public boolean equals(@Nullable Object o)
    {
        if (o == null) return false;
        if (getClass() != o.getClass()) return false;

        MessageSecurityCastleDto that = (MessageSecurityCastleDto) o;

        if (Objects.equals(this.getId(), that.getId()) == false) return false;

        List<MessageSecurityGateDto> gates1 = this.getGates();
        List<MessageSecurityGateDto> gates2 = that.getGates();

        if (Objects.equals(gates1.size(), gates2.size()) == false) return false;
        for (MessageSecurityGateDto gate : gates1) {
            if (gates2.contains(gate) == false) return false;
        }

        return true;
    }

    @Override
    public int hashCode()
    {
        int result = (int)0;
        if (this.getId() != null) result += this.getId().hashCode();
        return result;
    }

    public Map<String, String> getLookup() {
        Map<String, String> ret = this.lookupCache;
        if (ret != null) return ret;

        ret = new HashMap<>();
        for (MessageSecurityGateDto gate : this.getGates()) {
            ret.put(gate.getPublicKeyHash(), gate.getEncryptedText());
        }
        this.lookupCache = ret;

        return ret;
    }
}
