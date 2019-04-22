/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto.msg;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.google.flatbuffers.FlatBufferBuilder;
import com.google.gson.annotations.Expose;
import com.tokera.ate.constraints.PublicKeyConstraint;
import com.tokera.ate.dao.msg.*;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.security.Encryptor;

import java.io.Serializable;
import java.nio.ByteBuffer;
import java.util.Objects;

import com.tokera.ate.units.*;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;
import javax.ws.rs.WebApplicationException;

/**
 * Represents a public NTRU asymetric encryption key that can be placed on the distributed commit log
 */
@PublicKeyConstraint
@YamlTag("msg.public.key")
public class MessagePublicKeyDto extends MessageBaseDto implements Serializable
{
    private static final long serialVersionUID = -94567964466371784L;

    @Nullable
    protected transient MessagePublicKey fb;
    @Nullable
    protected transient Integer hashCache = null;

    @Expose
    @JsonProperty
    @MonotonicNonNull
    @Size(min=1, max=64)
    @Pattern(regexp = "^[a-zA-Z0-9_\\-\\:\\@\\.]+$")
    protected @Alias String alias;
    @Expose
    @JsonProperty
    @MonotonicNonNull
    @Size(min=43, max=43)
    @Pattern(regexp = "^(?:[A-Za-z0-9+\\/\\-_])*(?:[A-Za-z0-9+\\/\\-_]{2}==|[A-Za-z0-9+\\/\\-_]{3}=)?$")
    protected @Hash String publicKeyHash;
    @JsonIgnore
    protected transient @PEM byte @MonotonicNonNull [] publicKeyBytes;
    @Expose
    @JsonProperty
    @MonotonicNonNull
    protected @PEM String publicKey;

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
    
    private void copyOnWrite() {
        MessagePublicKey lfb = fb;
        if (lfb == null) return;

        this.hashCache = null;

        if (lfb.publicKeyLength() > 0) {
            ByteBuffer bb = lfb.publicKeyAsByteBuffer();
            if (bb == null) throw new WebApplicationException("Attached flat buffer does not have any key bytes.");

            @Secret byte [] keyBytes = new byte[bb.remaining()];
            bb.get(keyBytes);
            this.publicKeyBytes = keyBytes;
            this.publicKey = Base64.encodeBase64URLSafeString(keyBytes);
        } else {
            throw new WebApplicationException("Attached flat buffer does not have any key bytes.");
        }

        @Hash String hash = lfb.publicKeyHash();
        if (hash == null) {
            byte[] publicKeyBytes = this.publicKeyBytes;
            hash = Base64.encodeBase64URLSafeString(Encryptor.hashShaStatic(null, publicKeyBytes));
        }
        this.publicKeyHash = hash;

        @Alias String alias = lfb.alias();
        if (alias != null) {
            this.alias = alias;
        }

        fb = null;
    }
    
    public MessagePublicKeyDto(MessagePublicKeyDto key) {
        @Alias String alias = key.getAlias();
        if (alias != null) {
            this.alias = alias;
        }

        @PEM String publicKey = key.getPublicKey();
        if (publicKey != null) {
            this.publicKeyBytes = Base64.decodeBase64(publicKey);
            this.publicKey = publicKey;
        }

        @Hash String hash = key.getPublicKeyHash();
        if (hash != null) {
            this.publicKeyHash = hash;
        }
    }
    
    public MessagePublicKeyDto(@PEM String publicKey, @Hash String publicKeyHash) {
        this.publicKeyBytes = Base64.decodeBase64(publicKey);
        this.publicKey = publicKey;
        this.publicKeyHash = publicKeyHash;
    }
    
    public MessagePublicKeyDto(@PEM String publicKey) {
        this.publicKeyBytes = Base64.decodeBase64(publicKey);
        this.publicKey = publicKey;
        this.publicKeyHash = Base64.encodeBase64URLSafeString(Encryptor.hashShaStatic(null, this.publicKeyBytes));
    }
    
    public MessagePublicKeyDto(@PEM byte[] publicKey, @Hash String publicKeyHash) {
        this.publicKeyBytes = publicKey;
        this.publicKeyHash = publicKeyHash;
        this.publicKey = Base64.encodeBase64URLSafeString(publicKey);
    }

    public MessagePublicKeyDto(@PEM byte[] publicKeyBytes) {
        this.publicKeyBytes = publicKeyBytes;
        this.publicKey = Base64.encodeBase64URLSafeString(publicKeyBytes);
        this.publicKeyHash = Base64.encodeBase64URLSafeString(Encryptor.hashShaStatic(null, publicKeyBytes));
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
            @Hash String v = lfb.publicKeyHash();
            if (v != null) return v;
        }
        @Hash String ret = this.publicKeyHash;
        if (ret == null) {
            byte[] publicKeyBytes = this.getPublicKeyBytes();
            if (publicKeyBytes == null) return null;
            return Base64.encodeBase64URLSafeString(Encryptor.hashShaStatic(null, publicKeyBytes));
        }
        return ret;
    }

    public void setPublicKeyHash(@Hash String hash) {
        copyOnWrite();
        this.hashCache = null;
        this.publicKeyHash = hash;
    }

    private @Nullable @PEM String getPublicKeyInternal() {
        MessagePublicKey lfb = fb;
        if (lfb != null) {
            ByteBuffer bb = lfb.publicKeyAsByteBuffer();
            if (bb != null) {
                byte[] bytes = new byte[bb.remaining()];
                bb.get(bytes);
                return Base64.encodeBase64URLSafeString(bytes);
            }
        }

        return this.publicKey;
    }

    public @Nullable @PEM String getPublicKey() {
        @PEM String ret = getPublicKeyInternal();
        if (ret == null) {
            byte[] bytes = this.getPublicKeyBytesInternal();
            if (bytes == null) return null;
            return Base64.encodeBase64URLSafeString(bytes);
        }
        return ret;
    }

    public void setPublicKey(@PEM String publicKey) {
        copyOnWrite();
        this.hashCache = null;
        this.publicKeyBytes = Base64.decodeBase64(publicKey);
        this.publicKey = publicKey;
    }

    @JsonIgnore
    private @PEM byte @Nullable [] getPublicKeyBytesInternal() {
        MessagePublicKey lfb = fb;
        if (lfb != null) {
            ByteBuffer bb = lfb.publicKeyAsByteBuffer();
            if (bb != null) {
                byte[] bytes = new byte[bb.remaining()];
                bb.get(bytes);
                return bytes;
            }
        }
        return this.publicKeyBytes;
    }

    @JsonIgnore
    public @PEM byte @Nullable [] getPublicKeyBytes() {
        @PEM byte [] ret = getPublicKeyBytesInternal();
        if (ret == null) {
            @PEM String publicKey64 = this.getPublicKeyInternal();
            if (publicKey64 != null) ret = Base64.decodeBase64(publicKey64);
        }
        return ret;
    }
    
    @Override
    public int flatBuffer(FlatBufferBuilder fbb)
    {
        byte[] bytesPublicKey = this.getPublicKeyBytes();
        int offsetPublicKey = -1;
        if (bytesPublicKey != null) {
            offsetPublicKey = MessagePublicKey.createPublicKeyVector(fbb, bytesPublicKey);
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
        if (offsetPublicKey >= 0) {
            MessagePublicKey.addPublicKey(fbb, offsetPublicKey);
        }
        if (offsetPublicKeyHash >= 0) {
            MessagePublicKey.addPublicKeyHash(fbb, offsetPublicKeyHash);
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

        if (Objects.equals(this.alias, that.alias) == false) return false;
        if (Objects.equals(this.publicKeyHash, that.publicKeyHash) == false) return false;

        return true;
    }

    @Override
    public int hashCode()
    {
        Integer ret = this.hashCache;
        if (ret != null) return ret.intValue();

        ret = (int)0;
        if (this.alias != null) ret += this.alias.hashCode();
        if (this.publicKeyHash != null) ret += this.publicKeyHash.hashCode();

        this.hashCache = ret;
        return ret;
    }
}
