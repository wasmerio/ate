/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto.msg;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.google.common.base.Objects;
import com.tokera.ate.common.CopyOnWrite;
import com.tokera.ate.constraints.PrivateKeyConstraint;
import com.tokera.ate.dao.msg.*;
import com.google.flatbuffers.FlatBufferBuilder;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.security.Encryptor;
import com.tokera.ate.units.*;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.validation.ConstraintValidator;
import javax.validation.ConstraintValidatorContext;
import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;
import javax.ws.rs.WebApplicationException;
import java.io.Serializable;
import java.nio.ByteBuffer;

/**
 * Represents a private NTRU asymetric encryption key that can be placed on the distributed commit log
 */
@PrivateKeyConstraint
@YamlTag("msg.private.key")
public class MessagePrivateKeyDto extends MessagePublicKeyDto implements Serializable, ConstraintValidator, CopyOnWrite {

    private static final long serialVersionUID = -75643860128199913L;

    private transient @Nullable MessagePrivateKey pfb;

    @JsonProperty
    @MonotonicNonNull
    @Size(min=43, max=43)
    @Pattern(regexp = "^(?:[A-Za-z0-9+\\/\\-_])*(?:[A-Za-z0-9+\\/\\-_]{2}==|[A-Za-z0-9+\\/\\-_]{3}=)?$")
    private @Hash String privateKeyHash;
    @JsonIgnore
    private transient @Secret byte @MonotonicNonNull [] privateKeyBytes;
    @JsonProperty
    @MonotonicNonNull
    @Size(min = 2)
    private @Secret String privateKey;

    @Deprecated
    public MessagePrivateKeyDto() {
    }
    
    public MessagePrivateKeyDto(MessagePrivateKeyDto val) {
        super(val);

        byte[] publicKeyBytes = val.getPublicKeyBytes();
        if (publicKeyBytes != null) this.publicKeyBytes = publicKeyBytes;

        @Hash String publicKeyHash = val.getPublicKeyHash();
        if (publicKeyHash != null) this.publicKeyHash = publicKeyHash;

        @PEM String publicKey = val.getPublicKey();
        if (publicKey != null) this.publicKey = publicKey;

        byte[] privateKeyBytes = val.getPrivateKeyBytes();
        if (privateKeyBytes != null) this.privateKeyBytes = privateKeyBytes;

        @Hash String privateKeyHash = val.getPrivateKeyHash();
        if (privateKeyHash != null) this.privateKeyHash = privateKeyHash;

        @PEM String privateKey = val.getPrivateKey();
        if (privateKey != null) this.privateKey = privateKey;

        @Alias String alias = val.getAlias();
        if (alias != null) this.alias = alias;
    }
    
    public MessagePrivateKeyDto(MessagePrivateKey val) {
        super(val);

        pfb = val;
    }
    
    public MessagePrivateKeyDto(@PEM String publicKey, @Secret String privateKey) {
        super(publicKey);
        this.privateKeyBytes = Base64.decodeBase64(privateKey);
        this.privateKeyHash = Base64.encodeBase64URLSafeString(Encryptor.hashShaStatic(null, this.privateKeyBytes));
        this.privateKey = privateKey;
    }
    
    public MessagePrivateKeyDto(@PEM String publicKey, @Hash String publicKeyHash, @Secret String privateKey, @Hash String privateKeyHash) {
        super(publicKey, publicKeyHash);
        this.privateKeyBytes = Base64.decodeBase64(privateKey);
        this.privateKeyHash = privateKeyHash;
        this.privateKey = privateKey;
    }
    
    public MessagePrivateKeyDto(@PEM byte[] publicKey, @Hash String publicKeyHash, @Secret byte[] privateKey, @Hash String privateKeyHash) {
        super(publicKey, publicKeyHash);
        this.privateKeyBytes = privateKey;
        this.privateKeyHash = privateKeyHash;
        this.privateKey = Base64.encodeBase64URLSafeString(privateKey);
    }

    public MessagePrivateKeyDto(@PEM byte[] publicKeyBytes, @Secret byte[] privateKeyBytes) {
        super(publicKeyBytes);
        this.privateKeyBytes = privateKeyBytes;
        this.privateKey = org.apache.commons.codec.binary.Base64.encodeBase64URLSafeString(privateKeyBytes);
        this.privateKeyHash = org.apache.commons.codec.binary.Base64.encodeBase64URLSafeString(Encryptor.hashShaStatic(null, privateKeyBytes));
    }

    @Override
    public void copyOnWrite() {
        super.copyOnWrite();

        MessagePrivateKey lfb = pfb;
        if (lfb == null) return;

        this.hashCache = null;

        if (lfb.privateKeyLength() > 0) {
            ByteBuffer bb = lfb.privateKeyAsByteBuffer();
            if (bb == null) throw new WebApplicationException("Attached flat buffer does not have any key bytes.");

            @Secret byte [] keyBytes = new byte[bb.remaining()];
            bb.get(keyBytes);
            this.privateKeyBytes = keyBytes;
            this.privateKey = Base64.encodeBase64URLSafeString(keyBytes);
            
        } else {
            throw new WebApplicationException("Attached flat buffer does not have any key bytes.");
        }

        @Hash String hash = lfb.privateKeyHash();
        if (hash == null) {
            byte[] privateKeyBytes = this.privateKeyBytes;
            hash = Base64.encodeBase64URLSafeString(Encryptor.hashShaStatic(null, privateKeyBytes));
        }
        this.privateKeyHash = hash;

        pfb = null;
    }
    
    public @Nullable @Hash String getPrivateKeyHash() {
        MessagePrivateKey lfb = pfb;
        if (lfb != null) {
            @Hash String v = lfb.privateKeyHash();
            if (v != null) return v;
        }

        @Hash String ret = this.privateKeyHash;
        if (ret == null) {
            byte[] privateKeyBytes = this.getPrivateKeyBytes();
            if (privateKeyBytes == null) return null;
            return Base64.encodeBase64URLSafeString(Encryptor.hashShaStatic(null, privateKeyBytes));
        }
        return ret;
    }

    public void setPrivateKeyHash(@Hash String hash) {
        copyOnWrite();
        this.hashCache = null;
        this.privateKeyHash = hash;
    }

    private @Nullable @Secret String getPrivateKeyInternal() {
        MessagePrivateKey lfb = pfb;
        if (lfb != null) {
            ByteBuffer bb = lfb.privateKeyAsByteBuffer();
            if (bb != null) {
                byte[] bytes = new byte[bb.remaining()];
                bb.get(bytes);
                return Base64.encodeBase64URLSafeString(bytes);
            }
        }
        return this.privateKey;
    }

    public @Nullable @Secret String getPrivateKey() {
        @Secret String ret = getPrivateKeyInternal();
        if (ret == null) {
            byte[] bytes = this.getPrivateKeyBytesInternal();
            if (bytes == null) return null;
            return Base64.encodeBase64URLSafeString(bytes);
        }
        return ret;
    }

    public void setPrivateKey(@Secret String privateKey) {
        copyOnWrite();
        this.hashCache = null;
        this.privateKeyBytes = Base64.decodeBase64(privateKey);
        this.privateKey = privateKey;
    }

    @JsonIgnore
    private @Secret byte @Nullable [] getPrivateKeyBytesInternal() {
        MessagePrivateKey lfb = pfb;
        if (lfb != null) {
            ByteBuffer bb = lfb.privateKeyAsByteBuffer();
            if (bb != null) {
                byte[] bytes = new byte[bb.remaining()];
                bb.get(bytes);
                return bytes;
            }
        }
        return this.privateKeyBytes;
    }

    @JsonIgnore
    public @Secret byte @Nullable [] getPrivateKeyBytes() {
        @Secret byte [] ret = getPrivateKeyBytesInternal();
        if (ret == null) {
            @Secret String privateKey64 = this.getPrivateKeyInternal();
            if (privateKey64 != null) ret = Base64.decodeBase64(privateKey64);
        }
        return ret;
    }
    
    public int privateKeyFlatBuffer(FlatBufferBuilder fbb)
    {
        byte[] bytesPrivateKey = this.getPrivateKeyBytes();
        int offsetPrivateKey = -1;
        if (bytesPrivateKey != null) {
            offsetPrivateKey = MessagePrivateKey.createPrivateKeyVector(fbb, bytesPrivateKey);
        }

        String strPrivateKeyHash = this.getPrivateKeyHash();
        int offsetPrivateKeyHash = -1;
        if (strPrivateKeyHash != null) {
            offsetPrivateKeyHash = fbb.createString(strPrivateKeyHash);
        }

        int offsetPublicKey = this.flatBuffer(fbb);
        
        MessagePrivateKey.startMessagePrivateKey(fbb);
        if (offsetPrivateKey >= 0) {
            MessagePrivateKey.addPrivateKey(fbb, offsetPrivateKey);
        }
        if (offsetPrivateKeyHash >= 0) {
            MessagePrivateKey.addPrivateKeyHash(fbb, offsetPrivateKeyHash);
        }
        if (offsetPublicKey >= 0) {
            MessagePrivateKey.addPublicKey(fbb, offsetPublicKey);
        }
        return MessagePrivateKey.endMessagePrivateKey(fbb);
    }
    
    public MessagePrivateKey createPrivateKeyFlatBuffer()
    {
        FlatBufferBuilder fbb = new FlatBufferBuilder();
        fbb.finish(privateKeyFlatBuffer(fbb));
        return MessagePrivateKey.getRootAsMessagePrivateKey(fbb.dataBuffer());
    }

    @Override
    public boolean equals(@Nullable Object o)
    {
        if (o == null) return false;
        if (getClass() != o.getClass()) return false;

        MessagePrivateKeyDto that = (MessagePrivateKeyDto) o;

        if (Objects.equal(this.alias, that.alias) == false) return false;
        if (Objects.equal(this.publicKeyHash, that.publicKeyHash) == false) return false;
        if (Objects.equal(this.privateKeyHash, that.privateKeyHash) == false) return false;

        return true;
    }

    @Override
    public int hashCode()
    {
        Integer ret = this.hashCache;
        if (ret != null) return ret.intValue();

        ret = 0;
        if (this.alias != null) ret += this.alias.hashCode();
        if (this.publicKeyHash != null) ret += this.publicKeyHash.hashCode();
        if (this.privateKeyHash != null) ret += this.privateKeyHash.hashCode();

        this.hashCache = ret;
        return ret;
    }

    @Override
    public boolean isValid(Object o, ConstraintValidatorContext constraintValidatorContext) {
        return false;
    }
}
