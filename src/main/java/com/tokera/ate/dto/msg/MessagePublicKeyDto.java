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
public class MessagePublicKeyDto extends MessageBaseDto implements Serializable, CopyOnWrite
{
    private static final long serialVersionUID = -94567964466371784L;

    protected transient @Nullable MessagePublicKey fb;
    protected transient @Nullable Integer hashCache = null;

    @JsonProperty
    @MonotonicNonNull
    @Size(min=1, max=64)
    @Pattern(regexp = "^[a-zA-Z0-9_\\-\\:\\@\\.]+$")
    protected @Alias String alias;
    @JsonProperty
    @MonotonicNonNull
    @Size(min=43, max=43)
    @Pattern(regexp = "^(?:[A-Za-z0-9+\\/\\-_])*(?:[A-Za-z0-9+\\/\\-_]{2}==|[A-Za-z0-9+\\/\\-_]{3}=)?$")
    protected @Hash String publicKeyHash;
    @JsonIgnore
    protected transient @PEM byte @MonotonicNonNull [] publicKeyBytes1;
    @JsonIgnore
    protected transient @PEM byte @MonotonicNonNull [] publicKeyBytes2;
    @JsonProperty
    @MonotonicNonNull
    protected @PEM String publicKey1;
    @JsonProperty
    @MonotonicNonNull
    protected @PEM String publicKey2;

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

    @Override
    public void copyOnWrite() {
        MessagePublicKey lfb = fb;
        if (lfb == null) return;

        this.hashCache = null;

        if (lfb.publicKey1Length() > 0) {
            ByteBuffer bb = lfb.publicKey1AsByteBuffer();
            if (bb == null) throw new WebApplicationException("Attached flat buffer does not have any key bytes.");

            @Secret byte [] keyBytes = new byte[bb.remaining()];
            bb.get(keyBytes);
            this.publicKeyBytes1 = keyBytes;
            this.publicKey1 = Base64.encodeBase64URLSafeString(keyBytes);
        } else {
            throw new WebApplicationException("Attached flat buffer does not have any key bytes.");
        }

        if (lfb.publicKey2Length() > 0) {
            ByteBuffer bb = lfb.publicKey2AsByteBuffer();
            if (bb == null) throw new WebApplicationException("Attached flat buffer does not have any key bytes.");

            @Secret byte [] keyBytes = new byte[bb.remaining()];
            bb.get(keyBytes);
            this.publicKeyBytes2 = keyBytes;
            this.publicKey2 = Base64.encodeBase64URLSafeString(keyBytes);
        } else {
            throw new WebApplicationException("Attached flat buffer does not have any key bytes.");
        }

        @Hash String hash = lfb.publicKeyHash();
        if (hash == null) {
            byte[] publicKeyBytes1 = this.getPublicKeyBytes1();
            byte[] publicKeyBytes2 = this.getPublicKeyBytes2();
            hash = Base64.encodeBase64URLSafeString(Encryptor.hashShaStatic(publicKeyBytes1, publicKeyBytes2));
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

        @PEM String publicKey1 = key.getPublicKey1();
        if (publicKey1 != null) {
            this.publicKeyBytes1 = Base64.decodeBase64(publicKey1);
            this.publicKey1 = publicKey1;
        }

        @PEM String publicKey2 = key.getPublicKey2();
        if (publicKey2 != null) {
            this.publicKeyBytes2 = Base64.decodeBase64(publicKey2);
            this.publicKey2 = publicKey2;
        }

        @Hash String hash = key.getPublicKeyHash();
        if (hash != null) {
            this.publicKeyHash = hash;
        }
    }
    
    public MessagePublicKeyDto(@PEM String publicKey1, @PEM String publicKey2, @Hash String publicKeyHash) {
        this.publicKeyBytes1 = Base64.decodeBase64(publicKey1);
        this.publicKey1 = publicKey1;
        this.publicKeyBytes2 = Base64.decodeBase64(publicKey2);
        this.publicKey2 = publicKey2;
        this.publicKeyHash = publicKeyHash;
    }
    
    public MessagePublicKeyDto(@PEM String publicKey1, @PEM String publicKey2) {
        this.publicKeyBytes1 = Base64.decodeBase64(publicKey1);
        this.publicKey1 = publicKey1;
        this.publicKeyBytes2 = Base64.decodeBase64(publicKey2);
        this.publicKey2 = publicKey2;
        this.publicKeyHash = Base64.encodeBase64URLSafeString(Encryptor.hashShaStatic(this.publicKeyBytes1, this.publicKeyBytes2));
    }
    
    public MessagePublicKeyDto(@PEM byte[] publicKey1, @PEM byte[] publicKey2, @Hash String publicKeyHash) {
        this.publicKeyBytes1 = publicKey1;
        this.publicKeyBytes2 = publicKey2;
        this.publicKeyHash = publicKeyHash;
        this.publicKey1 = Base64.encodeBase64URLSafeString(publicKey1);
        this.publicKey2 = Base64.encodeBase64URLSafeString(publicKey2);
    }

    public MessagePublicKeyDto(@PEM byte[] publicKeyBytes1, @PEM byte[] publicKeyBytes2) {
        this.publicKeyBytes1 = publicKeyBytes1;
        this.publicKey1 = Base64.encodeBase64URLSafeString(publicKeyBytes1);
        this.publicKeyBytes2 = publicKeyBytes2;
        this.publicKey2 = Base64.encodeBase64URLSafeString(publicKeyBytes2);
        this.publicKeyHash = Base64.encodeBase64URLSafeString(Encryptor.hashShaStatic(publicKeyBytes1, publicKeyBytes2));
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
            byte[] publicKeyBytes1 = this.getPublicKeyBytes1();
            byte[] publicKeyBytes2 = this.getPublicKeyBytes2();
            if (publicKeyBytes1 == null) return null;
            if (publicKeyBytes2 == null) return null;
            return Base64.encodeBase64URLSafeString(Encryptor.hashShaStatic(publicKeyBytes1, publicKeyBytes2));
        }
        return ret;
    }

    public void setPublicKeyHash(@Hash String hash) {
        copyOnWrite();
        this.hashCache = null;
        this.publicKeyHash = hash;
    }

    private @Nullable @PEM String getPublicKeyInternal1() {
        MessagePublicKey lfb = fb;
        if (lfb != null) {
            ByteBuffer bb = lfb.publicKey1AsByteBuffer();
            if (bb != null) {
                byte[] bytes = new byte[bb.remaining()];
                bb.get(bytes);
                return Base64.encodeBase64URLSafeString(bytes);
            }
        }

        return this.publicKey1;
    }

    private @Nullable @PEM String getPublicKeyInternal2() {
        MessagePublicKey lfb = fb;
        if (lfb != null) {
            ByteBuffer bb = lfb.publicKey2AsByteBuffer();
            if (bb != null) {
                byte[] bytes = new byte[bb.remaining()];
                bb.get(bytes);
                return Base64.encodeBase64URLSafeString(bytes);
            }
        }

        return this.publicKey2;
    }

    public @Nullable @PEM String getPublicKey1() {
        @PEM String ret = getPublicKeyInternal1();
        if (ret == null) {
            byte[] bytes = this.getPublicKeyBytesInternal1();
            if (bytes == null) return null;
            return Base64.encodeBase64URLSafeString(bytes);
        }
        return ret;
    }

    public @Nullable @PEM String getPublicKey2() {
        @PEM String ret = getPublicKeyInternal2();
        if (ret == null) {
            byte[] bytes = this.getPublicKeyBytesInternal2();
            if (bytes == null) return null;
            return Base64.encodeBase64URLSafeString(bytes);
        }
        return ret;
    }

    public void setPublicKey1(@PEM String publicKey) {
        copyOnWrite();
        this.hashCache = null;
        this.publicKeyBytes1 = Base64.decodeBase64(publicKey);
        this.publicKey1 = publicKey;
    }

    public void setPublicKey2(@PEM String publicKey) {
        copyOnWrite();
        this.hashCache = null;
        this.publicKeyBytes2 = Base64.decodeBase64(publicKey);
        this.publicKey2 = publicKey;
    }

    @JsonIgnore
    private @PEM byte @Nullable [] getPublicKeyBytesInternal1() {
        MessagePublicKey lfb = fb;
        if (lfb != null) {
            ByteBuffer bb = lfb.publicKey1AsByteBuffer();
            if (bb != null) {
                byte[] bytes = new byte[bb.remaining()];
                bb.get(bytes);
                return bytes;
            }
        }
        return this.publicKeyBytes1;
    }

    @JsonIgnore
    private @PEM byte @Nullable [] getPublicKeyBytesInternal2() {
        MessagePublicKey lfb = fb;
        if (lfb != null) {
            ByteBuffer bb = lfb.publicKey2AsByteBuffer();
            if (bb != null) {
                byte[] bytes = new byte[bb.remaining()];
                bb.get(bytes);
                return bytes;
            }
        }
        return this.publicKeyBytes2;
    }

    @JsonIgnore
    public @PEM byte @Nullable [] getPublicKeyBytes1() {
        @PEM byte [] ret = getPublicKeyBytesInternal1();
        if (ret == null) {
            @PEM String publicKey64 = this.getPublicKeyInternal1();
            if (publicKey64 != null) ret = Base64.decodeBase64(publicKey64);
        }
        return ret;
    }

    @JsonIgnore
    public @PEM byte @Nullable [] getPublicKeyBytes2() {
        @PEM byte [] ret = getPublicKeyBytesInternal2();
        if (ret == null) {
            @PEM String publicKey64 = this.getPublicKeyInternal2();
            if (publicKey64 != null) ret = Base64.decodeBase64(publicKey64);
        }
        return ret;
    }
    
    @Override
    public int flatBuffer(FlatBufferBuilder fbb)
    {
        byte[] bytesPublicKey1 = this.getPublicKeyBytes1();
        int offsetPublicKey1 = -1;
        if (bytesPublicKey1 != null) {
            offsetPublicKey1 = MessagePublicKey.createPublicKey1Vector(fbb, bytesPublicKey1);
        }

        byte[] bytesPublicKey2 = this.getPublicKeyBytes2();
        int offsetPublicKey2 = -1;
        if (bytesPublicKey2 != null) {
            offsetPublicKey2 = MessagePublicKey.createPublicKey2Vector(fbb, bytesPublicKey2);
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
        if (offsetPublicKey1 >= 0) {
            MessagePublicKey.addPublicKey1(fbb, offsetPublicKey1);
        }
        if (offsetPublicKey2 >= 0) {
            MessagePublicKey.addPublicKey2(fbb, offsetPublicKey2);
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
