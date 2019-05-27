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
    private transient @Secret byte @MonotonicNonNull [] privateKeyBytes1;
    @JsonIgnore
    private transient @Secret byte @MonotonicNonNull [] privateKeyBytes2;
    @JsonProperty
    @MonotonicNonNull
    @Size(min = 2)
    private @Secret String privateKey1;
    @JsonProperty
    @MonotonicNonNull
    @Size(min = 2)
    private @Secret String privateKey2;

    @Deprecated
    public MessagePrivateKeyDto() {
    }
    
    public MessagePrivateKeyDto(MessagePrivateKeyDto val) {
        super(val);

        byte[] publicKeyBytes1 = val.getPublicKeyBytes1();
        if (publicKeyBytes1 != null) this.publicKeyBytes1 = publicKeyBytes1;

        byte[] publicKeyBytes2 = val.getPublicKeyBytes2();
        if (publicKeyBytes2 != null) this.publicKeyBytes2 = publicKeyBytes2;

        @Hash String publicKeyHash = val.getPublicKeyHash();
        if (publicKeyHash != null) this.publicKeyHash = publicKeyHash;

        @PEM String publicKey1 = val.getPublicKey1();
        if (publicKey1 != null) this.publicKey1 = publicKey1;

        @PEM String publicKey2 = val.getPublicKey2();
        if (publicKey2 != null) this.publicKey2 = publicKey2;

        byte[] privateKeyBytes1 = val.getPrivateKeyBytes1();
        if (privateKeyBytes1 != null) this.privateKeyBytes1 = privateKeyBytes1;

        byte[] privateKeyBytes2 = val.getPrivateKeyBytes2();
        if (privateKeyBytes2 != null) this.privateKeyBytes2 = privateKeyBytes2;

        @Hash String privateKeyHash = val.getPrivateKeyHash();
        if (privateKeyHash != null) this.privateKeyHash = privateKeyHash;

        @PEM String privateKey1 = val.getPrivateKey1();
        if (privateKey1 != null) this.privateKey1 = privateKey1;

        @PEM String privateKey2 = val.getPrivateKey2();
        if (privateKey2 != null) this.privateKey2 = privateKey2;


        @Alias String alias = val.getAlias();
        if (alias != null) this.alias = alias;
    }
    
    public MessagePrivateKeyDto(MessagePrivateKey val) {
        super(val);

        pfb = val;
    }
    
    public MessagePrivateKeyDto(@PEM String publicKey1, @PEM String publicKey2, @Secret String privateKey1, @Secret String privateKey2) {
        super(publicKey1, publicKey2);
        this.privateKeyBytes1 = Base64.decodeBase64(privateKey1);
        this.privateKeyBytes2 = Base64.decodeBase64(privateKey2);
        this.privateKeyHash = Base64.encodeBase64URLSafeString(Encryptor.hashShaStatic(this.privateKeyBytes1, this.privateKeyBytes2));
        this.privateKey1 = privateKey1;
        this.privateKey2 = privateKey2;
    }
    
    public MessagePrivateKeyDto(@PEM String publicKey1, @PEM String publicKey2, @Hash String publicKeyHash, @Secret String privateKey1, @Secret String privateKey2, @Hash String privateKeyHash) {
        super(publicKey1, publicKey2, publicKeyHash);
        this.privateKeyBytes1 = Base64.decodeBase64(privateKey1);
        this.privateKeyBytes2 = Base64.decodeBase64(privateKey2);
        this.privateKeyHash = privateKeyHash;
        this.privateKey1 = privateKey1;
        this.privateKey2 = privateKey2;
    }
    
    public MessagePrivateKeyDto(@PEM byte[] publicKey1, @PEM byte[] publicKey2, @Hash String publicKeyHash, @Secret byte[] privateKey1, @Secret byte[] privateKey2, @Hash String privateKeyHash) {
        super(publicKey1, publicKey2, publicKeyHash);
        this.privateKeyBytes1 = privateKey1;
        this.privateKeyBytes2 = privateKey2;
        this.privateKeyHash = privateKeyHash;
        this.privateKey1 = Base64.encodeBase64URLSafeString(privateKey1);
        this.privateKey2 = Base64.encodeBase64URLSafeString(privateKey2);
    }

    public MessagePrivateKeyDto(@PEM byte[] publicKeyBytes1, @PEM byte[] publicKeyBytes2, @Secret byte[] privateKeyBytes1, @Secret byte[] privateKeyBytes2) {
        super(publicKeyBytes1, publicKeyBytes2);
        this.privateKeyBytes1 = privateKeyBytes1;
        this.privateKeyBytes2 = privateKeyBytes2;
        this.privateKey1 = org.apache.commons.codec.binary.Base64.encodeBase64URLSafeString(privateKeyBytes1);
        this.privateKey2 = org.apache.commons.codec.binary.Base64.encodeBase64URLSafeString(privateKeyBytes2);
        this.privateKeyHash = org.apache.commons.codec.binary.Base64.encodeBase64URLSafeString(Encryptor.hashShaStatic(privateKeyBytes1, privateKeyBytes2));
    }

    @Override
    public void copyOnWrite() {
        super.copyOnWrite();

        MessagePrivateKey lfb = pfb;
        if (lfb == null) return;

        this.hashCache = null;

        if (lfb.privateKey1Length() > 0) {
            ByteBuffer bb = lfb.privateKey1AsByteBuffer();
            if (bb == null) throw new WebApplicationException("Attached flat buffer does not have any key bytes.");

            @Secret byte [] keyBytes = new byte[bb.remaining()];
            bb.get(keyBytes);
            this.privateKeyBytes1 = keyBytes;
            this.privateKey1 = Base64.encodeBase64URLSafeString(keyBytes);
            
        } else {
            throw new WebApplicationException("Attached flat buffer does not have any key bytes.");
        }

        if (lfb.privateKey2Length() > 0) {
            ByteBuffer bb = lfb.privateKey2AsByteBuffer();
            if (bb == null) throw new WebApplicationException("Attached flat buffer does not have any key bytes.");

            @Secret byte [] keyBytes = new byte[bb.remaining()];
            bb.get(keyBytes);
            this.privateKeyBytes2 = keyBytes;
            this.privateKey2 = Base64.encodeBase64URLSafeString(keyBytes);

        } else {
            throw new WebApplicationException("Attached flat buffer does not have any key bytes.");
        }

        @Hash String hash = lfb.privateKeyHash();
        if (hash == null) {
            byte[] privateKeyBytes1 = this.privateKeyBytes1;
            byte[] privateKeyBytes2 = this.privateKeyBytes2;
            hash = Base64.encodeBase64URLSafeString(Encryptor.hashShaStatic(privateKeyBytes1, privateKeyBytes2));
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
            byte[] privateKeyBytes1 = this.getPrivateKeyBytes1();
            byte[] privateKeyBytes2 = this.getPrivateKeyBytes2();
            if (privateKeyBytes1 == null) return null;
            if (privateKeyBytes2 == null) return null;
            return Base64.encodeBase64URLSafeString(Encryptor.hashShaStatic(privateKeyBytes1, privateKeyBytes2));
        }
        return ret;
    }

    public void setPrivateKeyHash(@Hash String hash) {
        copyOnWrite();
        this.hashCache = null;
        this.privateKeyHash = hash;
    }

    private @Nullable @Secret String getPrivateKeyInternal1() {
        MessagePrivateKey lfb = pfb;
        if (lfb != null) {
            ByteBuffer bb = lfb.privateKey1AsByteBuffer();
            if (bb != null) {
                byte[] bytes = new byte[bb.remaining()];
                bb.get(bytes);
                return Base64.encodeBase64URLSafeString(bytes);
            }
        }
        return this.privateKey1;
    }

    private @Nullable @Secret String getPrivateKeyInternal2() {
        MessagePrivateKey lfb = pfb;
        if (lfb != null) {
            ByteBuffer bb = lfb.privateKey2AsByteBuffer();
            if (bb != null) {
                byte[] bytes = new byte[bb.remaining()];
                bb.get(bytes);
                return Base64.encodeBase64URLSafeString(bytes);
            }
        }
        return this.privateKey2;
    }

    public @Nullable @Secret String getPrivateKey1() {
        @Secret String ret = getPrivateKeyInternal1();
        if (ret == null) {
            byte[] bytes = this.getPrivateKeyBytesInternal1();
            if (bytes == null) return null;
            return Base64.encodeBase64URLSafeString(bytes);
        }
        return ret;
    }

    public void setPrivateKey1(@Secret String privateKey) {
        copyOnWrite();
        this.hashCache = null;
        this.privateKeyBytes1 = Base64.decodeBase64(privateKey);
        this.privateKey1 = privateKey;
    }

    public @Nullable @Secret String getPrivateKey2() {
        @Secret String ret = getPrivateKeyInternal2();
        if (ret == null) {
            byte[] bytes = this.getPrivateKeyBytesInternal2();
            if (bytes == null) return null;
            return Base64.encodeBase64URLSafeString(bytes);
        }
        return ret;
    }

    public void setPrivateKey2(@Secret String privateKey) {
        copyOnWrite();
        this.hashCache = null;
        this.privateKeyBytes2 = Base64.decodeBase64(privateKey);
        this.privateKey2 = privateKey;
    }

    @JsonIgnore
    private @Secret byte @Nullable [] getPrivateKeyBytesInternal1() {
        MessagePrivateKey lfb = pfb;
        if (lfb != null) {
            ByteBuffer bb = lfb.privateKey1AsByteBuffer();
            if (bb != null) {
                byte[] bytes = new byte[bb.remaining()];
                bb.get(bytes);
                return bytes;
            }
        }
        return this.privateKeyBytes1;
    }

    @JsonIgnore
    private @Secret byte @Nullable [] getPrivateKeyBytesInternal2() {
        MessagePrivateKey lfb = pfb;
        if (lfb != null) {
            ByteBuffer bb = lfb.privateKey2AsByteBuffer();
            if (bb != null) {
                byte[] bytes = new byte[bb.remaining()];
                bb.get(bytes);
                return bytes;
            }
        }
        return this.privateKeyBytes2;
    }

    @JsonIgnore
    public @Secret byte @Nullable [] getPrivateKeyBytes1() {
        @Secret byte [] ret = getPrivateKeyBytesInternal1();
        if (ret == null) {
            @Secret String privateKey64 = this.getPrivateKeyInternal1();
            if (privateKey64 != null) ret = Base64.decodeBase64(privateKey64);
        }
        return ret;
    }

    @JsonIgnore
    public @Secret byte @Nullable [] getPrivateKeyBytes2() {
        @Secret byte [] ret = getPrivateKeyBytesInternal2();
        if (ret == null) {
            @Secret String privateKey64 = this.getPrivateKeyInternal2();
            if (privateKey64 != null) ret = Base64.decodeBase64(privateKey64);
        }
        return ret;
    }
    
    public int privateKeyFlatBuffer(FlatBufferBuilder fbb)
    {
        byte[] bytesPrivateKey1 = this.getPrivateKeyBytes1();
        int offsetPrivateKey1 = -1;
        if (bytesPrivateKey1 != null) {
            offsetPrivateKey1 = MessagePrivateKey.createPrivateKey1Vector(fbb, bytesPrivateKey1);
        }

        byte[] bytesPrivateKey2 = this.getPrivateKeyBytes2();
        int offsetPrivateKey2 = -1;
        if (bytesPrivateKey2 != null) {
            offsetPrivateKey2 = MessagePrivateKey.createPrivateKey2Vector(fbb, bytesPrivateKey2);
        }

        String strPrivateKeyHash = this.getPrivateKeyHash();
        int offsetPrivateKeyHash = -1;
        if (strPrivateKeyHash != null) {
            offsetPrivateKeyHash = fbb.createString(strPrivateKeyHash);
        }

        int offsetPublicKey = this.flatBuffer(fbb);
        
        MessagePrivateKey.startMessagePrivateKey(fbb);
        if (offsetPrivateKey1 >= 0) {
            MessagePrivateKey.addPrivateKey1(fbb, offsetPrivateKey1);
        }
        if (offsetPrivateKey2 >= 0) {
            MessagePrivateKey.addPrivateKey2(fbb, offsetPrivateKey2);
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
