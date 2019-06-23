/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto.msg;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.google.common.collect.Iterables;
import com.google.flatbuffers.FlatBufferBuilder;
import com.tokera.ate.common.CopyOnWrite;
import com.tokera.ate.common.ImmutalizableArrayList;
import com.tokera.ate.constraints.PublicKeyConstraint;
import com.tokera.ate.dao.msg.*;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.security.Encryptor;

import java.io.Serializable;
import java.nio.ByteBuffer;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;

import com.tokera.ate.units.*;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;
import javax.ws.rs.WebApplicationException;

/**
 * Represents a public NTRU asymetric encryption key that can be placed on the distributed commit log
 */
@Dependent
@PublicKeyConstraint
@YamlTag("msg.public.key")
public class MessagePublicKeyDto extends MessageBaseDto implements Serializable, CopyOnWrite
{
    private static final long serialVersionUID = 790094466708109400L;

    protected transient @Nullable MessagePublicKey fb;
    protected transient @Nullable Integer hashCache = null;

    @JsonProperty
    @MonotonicNonNull
    @Size(min=1, max=64)
    @Pattern(regexp = "^[a-zA-Z0-9_\\#\\-\\:\\@\\.]+$")
    protected @Alias String alias;
    @JsonProperty
    @MonotonicNonNull
    @Size(min=43, max=43)
    @Pattern(regexp = "^(?:[A-Za-z0-9+\\/\\-_])*(?:[A-Za-z0-9+\\/\\-_]{2}==|[A-Za-z0-9+\\/\\-_]{3}=)?$")
    protected @Hash String publicKeyHash;
    @JsonProperty
    protected ImmutalizableArrayList<MessageKeyPartDto> publicParts = new ImmutalizableArrayList<>();

    @Deprecated
    public MessagePublicKeyDto() {
    }
    
    public MessagePublicKeyDto(MessagePublicKey val) {
        fb = val;
    }

    public MessagePublicKeyDto(MessagePrivateKey val) {
        MessagePublicKey pubKey = val.publicKey();
        if (pubKey == null) throw new WebApplicationException("Private key has no public key attached to it.");
        fb = pubKey;
    }
    
    public MessagePublicKeyDto(MessageBase val) {
        if (val.msgType() == MessageType.MessagePublicKey) {
            fb = (MessagePublicKey)val.msg(new MessagePublicKey());
        } else {
            throw new WebApplicationException("Invalidate message type [expected=MessagePublicKey, actual=" + val.msgType() + "]");
        }
    }
    
    public MessagePublicKeyDto(MessagePublicKeyDto key) {
        this.alias = key.getAlias();
        this.publicKeyHash = key.getPublicKeyHash();

        this.publicParts.clear();
        for (MessageKeyPartDto part : key.getPublicParts()) {
            this.publicParts.add(new MessageKeyPartDto(part));
        }
    }

    public MessagePublicKeyDto(Iterable<MessageKeyPartDto> publicParts) {
        this.publicParts.clear();
        for (MessageKeyPartDto part : publicParts) {
            this.publicParts.add(new MessageKeyPartDto(part));
        }
        this.publicKeyHash = MessageKeyPartDto.computeHash(publicParts);
    }
    
    public MessagePublicKeyDto(Iterable<MessageKeyPartDto> publicParts, @Hash String publicKeyHash) {
        this.publicParts.clear();
        for (MessageKeyPartDto part : publicParts) {
            this.publicParts.add(new MessageKeyPartDto(part));
        }
        this.publicKeyHash = publicKeyHash;
    }

    @Override
    public void copyOnWrite() {
        this.hashCache = null;
        MessagePublicKey lfb = fb;
        if (lfb == null) return;

        this.publicParts.clear();
        if (lfb.partsLength() > 0) {
            for (int n = 0; n < lfb.partsLength(); n++) {
                MessageKeyPart part = lfb.parts(n);
                this.publicParts.add(new MessageKeyPartDto(part));
            }
        }

        this.publicKeyHash = lfb.hash();
        this.alias = lfb.alias();

        fb = null;
    }
    
    public @Nullable @Alias String getAlias() {
        MessagePublicKey lfb = fb;
        if (lfb != null) {
            @Alias String fbAlias = lfb.alias();
            if (fbAlias != null) {
                return fbAlias;
            }
        }
        return this.alias;
    }
    
    public MessagePublicKeyDto setAlias(@Alias String alias) {
        copyOnWrite();
        this.hashCache = null;
        this.alias = alias;
        return this;
    }
    
    public @Nullable @Hash String getPublicKeyHash() {
        MessagePublicKey lfb = fb;
        if (lfb != null) {
            @Hash String v = lfb.hash();
            if (v != null) return v;
        }
        @Hash String ret = this.publicKeyHash;
        if (ret == null) {
            ret = MessageKeyPartDto.computeHash(this.getPublicParts());
        }
        return ret;
    }

    public void setPublicKeyHash(@Hash String hash) {
        copyOnWrite();
        this.hashCache = null;
        this.publicKeyHash = hash;
    }

    public ImmutalizableArrayList<MessageKeyPartDto> getPublicParts() {
        MessagePublicKey lfb = fb;
        if (lfb != null) {
            ImmutalizableArrayList<MessageKeyPartDto> ret = new ImmutalizableArrayList<>();
            ret.clear();
            if (lfb.partsLength() > 0) {
                for (int n = 0; n < lfb.partsLength(); n++) {
                    MessageKeyPart part = lfb.parts(n);
                    ret.add(new MessageKeyPartDto(part));
                }
            }
            return ret;
        }

        return this.publicParts;
    }

    public void setPublicParts(ImmutalizableArrayList<MessageKeyPartDto> publicParts) {
        copyOnWrite();
        this.hashCache = null;
        this.publicParts = publicParts;
    }
    
    @Override
    public int flatBuffer(FlatBufferBuilder fbb)
    {
        Iterable<MessageKeyPartDto> publicParts = this.getPublicParts();
        int offsetPublicParts = -1;
        if (publicParts != null) {
            int size =  Iterables.size(publicParts);
            int[] partOffsets = new int[size];

            int n = 0;
            for (MessageKeyPartDto part : publicParts) {
                partOffsets[n++] = part.flatBuffer(fbb);
            }

            offsetPublicParts = MessagePublicKey.createPartsVector(fbb, partOffsets);
        }

        String strPublicKeyHash = this.getPublicKeyHash();
        int offsetPublicKeyHash = -1;
        if (strPublicKeyHash != null) {
            offsetPublicKeyHash = fbb.createString(strPublicKeyHash);
        }

        String strAlias = this.getAlias();
        int offsetAlias = -1;
        if (strAlias != null) {
            offsetAlias = fbb.createString(strAlias);
        }

        MessagePublicKey.startMessagePublicKey(fbb);
        if (offsetPublicParts > 0) {
            MessagePublicKey.addParts(fbb, offsetPublicParts);
        }
        if (offsetPublicKeyHash >= 0) {
            MessagePublicKey.addHash(fbb, offsetPublicKeyHash);
        }
        if (offsetAlias >= 0) {
            MessagePublicKey.addAlias(fbb, offsetAlias);
        }
        return MessagePublicKey.endMessagePublicKey(fbb);
    }
    
    public MessagePublicKey createFlatBuffer()
    {
        FlatBufferBuilder fbb = new FlatBufferBuilder();
        fbb.finish(flatBuffer(fbb));
        return MessagePublicKey.getRootAsMessagePublicKey(fbb.dataBuffer());
    }

    @Override
    public boolean equals(@Nullable Object o)
    {
        if (o == null) return false;
        if (getClass() != o.getClass()) return false;

        MessagePublicKeyDto that = (MessagePublicKeyDto) o;

        if (Objects.equals(this.getAlias(), that.getAlias()) == false) return false;
        if (Objects.equals(this.getPublicKeyHash(), that.getPublicKeyHash()) == false) return false;

        return true;
    }

    @Override
    public int hashCode()
    {
        Integer ret = this.hashCache;
        if (ret != null) return ret.intValue();

        ret = (int)0;

        String alias = this.getAlias();
        if (alias != null) {
            ret += alias.hashCode();
        }

        ret += this.getPublicKeyHash().hashCode();

        this.hashCache = ret;
        return ret;
    }
}
