/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto.msg;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.google.common.base.Objects;
import com.google.common.collect.Iterables;
import com.tokera.ate.common.CopyOnWrite;
import com.tokera.ate.common.ImmutalizableArrayList;
import com.tokera.ate.constraints.PrivateKeyConstraint;
import com.tokera.ate.dao.enumerations.KeyType;
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
    @JsonProperty
    private ImmutalizableArrayList<MessageKeyPartDto> privateParts = new ImmutalizableArrayList<>();

    @Deprecated
    public MessagePrivateKeyDto() {
    }

    public MessagePrivateKeyDto(MessagePrivateKeyDto key) {
        super(key);

        this.alias = key.getAlias();
        this.publicKeyHash = key.getPublicKeyHash();
        this.privateKeyHash = key.getPrivateKeyHash();

        this.publicParts.clear();
        for (MessageKeyPartDto part : key.getPublicParts()) {
            this.publicParts.add(new MessageKeyPartDto(part));
        }

        this.privateParts.clear();
        for (MessageKeyPartDto part : key.getPublicParts()) {
            this.privateParts.add(new MessageKeyPartDto(part));
        }
    }
    
    public MessagePrivateKeyDto(MessagePrivateKey val) {
        super(val);

        pfb = val;
    }

    public MessagePrivateKeyDto(Iterable<MessageKeyPartDto> publicParts, Iterable<MessageKeyPartDto> privateParts) {
        super(publicParts);

        this.privateParts.clear();
        for (MessageKeyPartDto part : privateParts) {
            this.privateParts.add(new MessageKeyPartDto(part));
        }
        this.privateKeyHash = MessageKeyPartDto.computeHash(privateParts);
    }

    public MessagePrivateKeyDto(Iterable<MessageKeyPartDto> publicParts, Iterable<MessageKeyPartDto> privateParts, @Hash String publicKeyHash, @Hash String privateKeyHash) {
        super(publicParts, publicKeyHash);

        this.privateParts.clear();
        for (MessageKeyPartDto part : privateParts) {
            this.privateParts.add(new MessageKeyPartDto(part));
        }
        this.privateKeyHash = publicKeyHash;
    }

    @Override
    public void copyOnWrite() {
        super.copyOnWrite();
        this.hashCache = null;

        MessagePrivateKey lfb = pfb;
        if (lfb == null) return;

        this.privateParts.clear();
        if (lfb.partsLength() > 0) {
            for (int n = 0; n < lfb.partsLength(); n++) {
                MessageKeyPart part = lfb.parts(n);
                this.privateParts.add(new MessageKeyPartDto(part));
            }
        }

        this.privateKeyHash = lfb.hash();

        pfb = null;
    }
    
    public @Nullable @Hash String getPrivateKeyHash() {
        MessagePrivateKey lfb = pfb;
        if (lfb != null) {
            @Hash String v = lfb.hash();
            if (v != null) return v;
        }

        @Hash String ret = this.privateKeyHash;
        if (ret == null) {
            ret = MessageKeyPartDto.computeHash(this.getPrivateParts());
        }
        return ret;
    }

    public void setPrivateKeyHash(@Hash String hash) {
        copyOnWrite();
        this.hashCache = null;
        this.privateKeyHash = hash;
    }

    public ImmutalizableArrayList<MessageKeyPartDto> getPrivateParts() {
        MessagePrivateKey lfb = pfb;
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

        return this.privateParts;
    }

    public void setPrivateParts(ImmutalizableArrayList<MessageKeyPartDto> privateParts) {
        copyOnWrite();
        this.hashCache = null;
        this.privateParts = privateParts;
    }
    
    public int privateKeyFlatBuffer(FlatBufferBuilder fbb)
    {
        Iterable<MessageKeyPartDto> privateParts = this.getPrivateParts();
        int offsetPrivateParts = -1;
        if (privateParts != null) {
            int size =  Iterables.size(privateParts);
            int[] partOffsets = new int[size];

            int n = 0;
            for (MessageKeyPartDto part : privateParts) {
                partOffsets[n++] = part.flatBuffer(fbb);
            }

            offsetPrivateParts = MessagePrivateKey.createPartsVector(fbb, partOffsets);
        }

        String strPrivateKeyHash = this.getPrivateKeyHash();
        int offsetPrivateKeyHash = -1;
        if (strPrivateKeyHash != null) {
            offsetPrivateKeyHash = fbb.createString(strPrivateKeyHash);
        }

        int offsetPublicKey = this.flatBuffer(fbb);
        
        MessagePrivateKey.startMessagePrivateKey(fbb);
        if (offsetPrivateParts >= 0) {
            MessagePrivateKey.addParts(fbb, offsetPrivateParts);
        }
        if (offsetPrivateKeyHash >= 0) {
            MessagePrivateKey.addHash(fbb, offsetPrivateKeyHash);
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

        if (java.util.Objects.equals(this.getAlias(), that.getAlias()) == false) return false;
        if (java.util.Objects.equals(this.getPublicKeyHash(), that.getPublicKeyHash()) == false) return false;
        if (java.util.Objects.equals(this.getPrivateKeyHash(), that.getPrivateKeyHash()) == false) return false;

        return true;
    }

    @Override
    public int hashCode()
    {
        Integer ret = this.hashCache;
        if (ret != null) return ret.intValue();

        ret = (int)0;
        ret += this.getAlias().hashCode();
        ret += this.getPublicKeyHash().hashCode();
        ret += this.getPrivateKeyHash().hashCode();

        this.hashCache = ret;
        return ret;
    }

    @Override
    public boolean isValid(Object o, ConstraintValidatorContext constraintValidatorContext) {
        return false;
    }

    public void addKeyPart(KeyType keyType, int keySize, Encryptor.KeyPairBytes pair)
    {
        copyOnWrite();
        this.hashCache = null;

        MessageKeyPartDto partPublic = new MessageKeyPartDto(keyType, keySize, pair.publicKey);
        MessageKeyPartDto partPrivate = new MessageKeyPartDto(keyType, keySize, pair.privateKey);
        this.publicParts.add(partPublic);
        this.privateParts.add(partPrivate);
    }
}
