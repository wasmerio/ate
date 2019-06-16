/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto.msg;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.google.flatbuffers.FlatBufferBuilder;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.common.CopyOnWrite;
import com.tokera.ate.dao.enumerations.KeyType;
import com.tokera.ate.dao.msg.*;
import com.tokera.ate.security.Encryptor;
import com.tokera.ate.units.PEM;
import com.tokera.ate.units.Secret;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import javax.ws.rs.WebApplicationException;
import java.io.Serializable;
import java.nio.ByteBuffer;
import java.util.Arrays;
import java.util.Objects;

/**
 * Represents part of an asymmetric encryption key pair
 */
@Dependent
@YamlTag("msg.part.key")
public class MessageKeyPartDto extends MessageBaseDto implements Serializable, CopyOnWrite
{
    private static final long serialVersionUID = -4895592870824999743L;

    protected transient @Nullable MessageKeyPart fb;
    protected transient @Nullable Integer hashCache = null;

    @JsonProperty
    protected KeyType type = KeyType.unknown;
    @JsonProperty
    protected int size = 0;
    @JsonIgnore
    protected transient @PEM byte @MonotonicNonNull [] keyBytes;
    @JsonProperty
    @MonotonicNonNull
    protected @PEM String key64;

    @Deprecated
    public MessageKeyPartDto() {
    }

    public MessageKeyPartDto(MessageKeyPart val) {
        fb = val;
    }

    public MessageKeyPartDto(MessageKeyPartDto key) {

        this.type = key.type;

        this.size = key.size;

        @PEM String key64 = key.getKey64();
        if (key64 != null) {
            this.keyBytes = Base64.decodeBase64(key64);
            this.key64 = key64;
        }
    }

    public MessageKeyPartDto(KeyType type, int size, @PEM String key64) {
        this.type = type;
        this.size = size;
        this.keyBytes = Base64.decodeBase64(key64);
        this.key64 = key64;
    }

    public MessageKeyPartDto(KeyType type, int size, @PEM byte[] keyBytes) {
        this.type = type;
        this.size = size;
        this.keyBytes = keyBytes;
        this.key64 = Base64.encodeBase64URLSafeString(keyBytes);
    }

    @Override
    public void copyOnWrite() {
        hashCache = null;
        MessageKeyPart lfb = fb;
        if (lfb == null) return;

        this.type = KeyType.get(lfb.type());

        this.size = lfb.size();

        if (lfb.keyLength() > 0) {
            ByteBuffer bb = lfb.keyAsByteBuffer();
            if (bb == null) throw new WebApplicationException("Attached flat buffer does not have any key bytes.");

            @Secret byte [] keyBytes = new byte[bb.remaining()];
            bb.get(keyBytes);
            this.keyBytes = keyBytes;
            this.key64 = Base64.encodeBase64URLSafeString(keyBytes);
        } else {
            throw new WebApplicationException("Attached flat buffer does not have any key bytes.");
        }

        fb = null;
    }

    public KeyType getType() {
        MessageKeyPart lfb = fb;
        if (lfb != null) {
            return KeyType.get(lfb.type());
        }
        return type;
    }

    public void setType(KeyType type) {
        copyOnWrite();
        this.type = type;
    }

    public int getSize() {
        MessageKeyPart lfb = fb;
        if (lfb != null) {
            return lfb.size();
        }
        return size;
    }

    public void setSize(int size) {
        copyOnWrite();
        this.size = size;
    }

    private @Nullable @PEM String getKeyInternal() {
        MessageKeyPart lfb = fb;
        if (lfb != null) {
            ByteBuffer bb = lfb.keyAsByteBuffer();
            if (bb != null) {
                byte[] bytes = new byte[bb.remaining()];
                bb.get(bytes);
                return Base64.encodeBase64URLSafeString(bytes);
            }
        }

        return this.key64;
    }

    public @Nullable @PEM String getKey64() {
        @PEM String ret = getKeyInternal();
        if (ret == null) {
            byte[] bytes = this.getKeyBytesInternal();
            if (bytes == null) return null;
            return Base64.encodeBase64URLSafeString(bytes);
        }
        return ret;
    }

    public void setKey64(@PEM String key64) {
        copyOnWrite();
        this.keyBytes = Base64.decodeBase64(key64);
        this.key64 = key64;
    }

    @JsonIgnore
    private @PEM byte @Nullable [] getKeyBytesInternal() {
        MessageKeyPart lfb = fb;
        if (lfb != null) {
            ByteBuffer bb = lfb.keyAsByteBuffer();
            if (bb != null) {
                byte[] bytes = new byte[bb.remaining()];
                bb.get(bytes);
                return bytes;
            }
        }
        return this.keyBytes;
    }

    @JsonIgnore
    public @PEM byte @Nullable [] getKeyBytes() {
        @PEM byte [] ret = getKeyBytesInternal();
        if (ret == null) {
            @PEM String publicKey64 = this.getKeyInternal();
            if (publicKey64 != null) ret = Base64.decodeBase64(publicKey64);
        }
        return ret;
    }
    
    @Override
    public int flatBuffer(FlatBufferBuilder fbb)
    {
        byte[] bytesKey = this.getKeyBytes();
        int offsetKey = -1;
        if (bytesKey != null) {
            offsetKey = MessageKeyPart.createKeyVector(fbb, bytesKey);
        }

        MessageKeyPart.startMessageKeyPart(fbb);
        MessageKeyPart.addType(fbb, (short)this.getType().getCode());
        MessageKeyPart.addSize(fbb, (short)this.getSize());
        if (offsetKey >= 0) {
            MessageKeyPart.addKey(fbb, offsetKey);
        }
        return MessageKeyPart.endMessageKeyPart(fbb);
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

        MessageKeyPartDto that = (MessageKeyPartDto) o;

        if (Objects.equals(this.getType(), that.getType()) == false) return false;
        if (Objects.equals(this.getSize(), that.getSize()) == false) return false;
        if (Arrays.equals(this.getKeyBytes(), that.getKeyBytes()) == false) return false;

        return true;
    }

    @Override
    public int hashCode()
    {
        Integer ret = this.hashCache;
        if (ret != null) return ret.intValue();

        ret = (int)0;
        ret += this.getType().hashCode();
        ret += Integer.hashCode(this.getSize());
        ret += Arrays.hashCode(this.getKeyBytes());

        this.hashCache = ret;
        return ret;
    }

    public static String computeHash(@Nullable Iterable<MessageKeyPartDto> _parts)
    {
        Iterable<MessageKeyPartDto> parts = _parts;
        if (parts == null) {
            throw new RuntimeException("The key parts to be hashed can not be empty or null.");
        }
        byte[] hash = null;
        for (MessageKeyPartDto part : parts) {
            @PEM byte[] keyBytes = part.getKeyBytes();
            if (keyBytes == null) {
                throw new RuntimeException("The key parts have missing data.");
            }
            hash = Encryptor.hashShaStatic(hash, keyBytes);
        }
        if (hash == null) {
            hash = new byte[0];
        }
        return Base64.encodeBase64URLSafeString(hash);
    }
}
