/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto.msg;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.google.flatbuffers.FlatBufferBuilder;
import com.tokera.ate.common.ByteBufferTools;
import com.tokera.ate.common.CopyOnWrite;
import com.tokera.ate.common.Immutalizable;
import com.tokera.ate.dao.msg.*;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.units.Hash;
import com.tokera.ate.units.PlainText;
import com.tokera.ate.units.Salt;
import com.tokera.ate.units.Signature;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;
import javax.ws.rs.WebApplicationException;
import java.io.Serializable;
import java.nio.ByteBuffer;

/**
 * Represents the digest of a data payload signed by an authorized user
 */
@Dependent
@YamlTag("msg.data.digest")
public class MessageDataDigestDto extends MessageBaseDto implements Serializable, CopyOnWrite, Immutalizable {

    private static final long serialVersionUID = 3992438221645570455L;

    // When running in copy-on-write mode
    private transient @Nullable MessageDataDigest fb;
    
    // Fields that are serialized
    @JsonProperty
    @MonotonicNonNull
    private @Salt String seed;                // Seed added to the digest calculation
    @JsonProperty
    @MonotonicNonNull
    private @Signature String signature;           // digitally signed digest of the payload _after_ it was encrypted
    @JsonProperty
    @MonotonicNonNull
    @Size(min=43, max=43)
    @Pattern(regexp = "^(?:[A-Za-z0-9+\\/\\-_])*(?:[A-Za-z0-9+\\/\\-_]{2}==|[A-Za-z0-9+\\/\\-_]{3}=)?$")
    private @Hash String digest;              // Digest of the header and its payload
    @JsonProperty
    @MonotonicNonNull
    @Size(min=43, max=43)
    @Pattern(regexp = "^(?:[A-Za-z0-9+\\/\\-_])*(?:[A-Za-z0-9+\\/\\-_]{2}==|[A-Za-z0-9+\\/\\-_]{3}=)?$")
    private @Hash String publicKeyHash;       // public key hash used in the verifiation process of the payload signature

    @JsonIgnore
    private transient boolean _immutable = false;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public MessageDataDigestDto() {
    }

    public MessageDataDigestDto(
            @Salt String seed,
            @Signature String signature,
            @Hash String digest,
            @Hash String publicKeyHash)
    {
        this.seed = seed;
        this.signature = signature;
        this.digest = digest;
        this.publicKeyHash = publicKeyHash;
    }
    
    public MessageDataDigestDto(MessageDataDigest val)
    {
        this.fb = val;
    }
    
    public void setFlatBuffer(MessageDataDigest val) {
        assert this._immutable == false;
        this.fb = val;
    }

    @Override
    public void copyOnWrite() {
        MessageDataDigest lfb = fb;
        if (lfb == null) return;

        if (lfb.seedLength() > 0) {
            ByteBuffer bb = lfb.seedAsByteBuffer();
            if (bb != null) {
                String v = ByteBufferTools.toBase64(bb);
                if (v != null) seed = v;
            }
        }
        if (lfb.signatureLength() > 0) {
            ByteBuffer bb = lfb.signatureAsByteBuffer();
            if (bb != null) {
                String v = ByteBufferTools.toBase64(bb);
                if (v != null) signature = v;
            }
        }
        if (lfb.digestLength() > 0) {
            ByteBuffer bb = lfb.digestAsByteBuffer();
            if (bb != null) {
                String v = ByteBufferTools.toBase64(bb);
                if (v != null) digest = v;
            }
        }
        String v = lfb.publicKeyHash();
        if (v != null) publicKeyHash = v;

        fb = null;
    }
    
    public @Nullable @Signature String getSignature() {
        MessageDataDigest lfb = fb;
        if (lfb != null) {
            if (lfb.signatureLength() > 0) {
                ByteBuffer bb = lfb.signatureAsByteBuffer();
                if (bb != null) {
                    @Signature String v = ByteBufferTools.toBase64(bb);
                    if (v != null) return v;
                }
            }
        }
        @Signature String ret = this.signature;
        if (ret == null) return null;
        return ret;
    }
    
    public @Nullable @Signature byte[] getSignatureBytes() {
        MessageDataDigest lfb = fb;
        if (lfb != null) {
            if (lfb.signatureLength() > 0) {
                ByteBuffer bb = lfb.signatureAsByteBuffer();
                if (bb != null) {
                    byte[] arr = new byte[bb.remaining()];
                    bb.get(arr);
                    return arr;
                }
            }
        }
        @Signature String ret = this.signature;
        if (ret == null) return null;
        return Base64.decodeBase64(ret);
    }

    public void setSignature(@Signature String signature) {
        assert this._immutable == false;
        copyOnWrite();
        this.signature = signature;
    }

    public @Nullable @Hash String getPublicKeyHash() {
        MessageDataDigest lfb = fb;
        if (lfb != null) {
            @Hash String v = lfb.publicKeyHash();
            if (v != null) return v;
        }
        String ret = this.publicKeyHash;
        if (ret == null) return null;
        return ret;
    }

    public void setPublicKeyHash(@Hash String hash) {
        assert this._immutable == false;
        copyOnWrite();
        this.publicKeyHash = hash;
    }
    
    public @Nullable @Salt String getSeed() {
        MessageDataDigest lfb = fb;
        if (lfb != null) {
            if (lfb.seedLength() > 0) {
                ByteBuffer bb = lfb.seedAsByteBuffer();
                if (bb != null) {
                    @Salt String v = ByteBufferTools.toBase64(bb);
                    if (v != null) return v;
                }
            }
        }
        return this.seed;
    }

    @JsonIgnore
    public @Salt String getSeedOrThrow() {
        @Salt String ret = this.getSeed();
        if (ret == null) throw new WebApplicationException("MessageDataDigest has no seed bytes attached.");
        return ret;
    }

    public void setSeed(@Salt String seed) {
        assert this._immutable == false;
        copyOnWrite();
        this.seed = seed;
    }
    
    public @Nullable @PlainText byte[] getSeedBytes() {
        MessageDataDigest lfb = fb;
        if (lfb != null) {
            if (lfb.seedLength() > 0) {
                ByteBuffer bb = lfb.seedAsByteBuffer();
                if (bb != null) {
                    byte[] arr = new byte[bb.remaining()];
                    bb.get(arr);
                    return arr;
                }
            }
        }
        @PlainText String ret = this.seed;
        if (ret == null) return null;
        return Base64.decodeBase64(ret);
    }

    @JsonIgnore
    public @PlainText byte[] getSeedBytesOrThrow() {
        @PlainText byte[] ret = this.getSeedBytes();
        if (ret == null) throw new WebApplicationException("MessageDataDigest has no seed bytes attached.");
        return ret;
    }
    
    public @Nullable @Hash String getDigest() {
        MessageDataDigest lfb = fb;
        if (lfb != null) {
            if (lfb.digestLength() > 0) {
                ByteBuffer bb = lfb.digestAsByteBuffer();
                if (bb != null) {
                    @Hash String v = ByteBufferTools.toBase64(bb);
                    if (v != null) return v;
                }
            }
        }

        return this.digest;
    }

    @JsonIgnore
    public @Hash String getDigestOrThrow() {
        @Hash String ret = this.getDigest();
        if (ret == null) throw new WebApplicationException("MessageDataDigest has no digest bytes attached.");
        return ret;
    }

    public @Nullable @Hash byte[] getDigestBytes() {
        MessageDataDigest lfb = fb;
        if (lfb != null) {
            if (lfb.digestLength() > 0) {
                ByteBuffer bb = lfb.digestAsByteBuffer();
                if (bb != null) {
                    byte[] arr = new byte[bb.remaining()];
                    bb.get(arr);
                    return arr;
                }
            }
        }
        @Hash String ret = this.digest;
        if (ret == null) return null;
        return Base64.decodeBase64(ret);
    }

    @JsonIgnore
    public @Hash byte[] getDigestBytesOrThrow() {
        @Hash byte[] ret = this.getDigestBytes();
        if (ret == null) throw new WebApplicationException("MessageDataDigest has no digest bytes attached.");
        return ret;
    }
    
    public void setDigest(@Hash String digest) {
        assert this._immutable == false;
        copyOnWrite();
        this.digest = digest;
    }
    
    @Override
    public int flatBuffer(FlatBufferBuilder fbb)
    {
        int offsetSeed = -1;
        int offsetSignature = -1;
        int offsetDigest = -1;
        int offsetPublicKeyHash = -1;
        
        String seedStr = this.getSeed();
        if (seedStr != null && seedStr.length() > 0) {
            offsetSeed = MessageDataDigest.createSeedVector(fbb, Base64.decodeBase64(seedStr));
        }
        
        String sigStr1 = this.getSignature();
        if (sigStr1 != null && sigStr1.length() > 0) {
            offsetSignature = MessageDataDigest.createSignatureVector(fbb, Base64.decodeBase64(sigStr1));
        }
        
        String digestStr = this.getDigest();
        if (digestStr != null && digestStr.length() > 0) {
            offsetDigest = MessageDataDigest.createDigestVector(fbb, Base64.decodeBase64(digestStr));
        }
        
        String publicKeyHashStr = this.getPublicKeyHash();
        if (publicKeyHashStr != null && publicKeyHashStr.length() > 0) {
            offsetPublicKeyHash = fbb.createString(publicKeyHashStr);
        }
        
        MessageDataDigest.startMessageDataDigest(fbb);
        if (offsetSeed >= 0) MessageDataDigest.addSeed(fbb, offsetSeed);
        if (offsetSignature >= 0) MessageDataDigest.addSignature(fbb, offsetSignature);
        if (offsetDigest >= 0) MessageDataDigest.addDigest(fbb, offsetDigest);
        if (offsetPublicKeyHash >= 0) MessageDataDigest.addPublicKeyHash(fbb, offsetPublicKeyHash);
        return MessageDataDigest.endMessageDataDigest(fbb);
    }
    
    public MessageDataDigest createFlatBuffer()
    {
        FlatBufferBuilder fbb = new FlatBufferBuilder();
        fbb.finish(flatBuffer(fbb));            
        return MessageDataDigest.getRootAsMessageDataDigest(fbb.dataBuffer());
    }

    public void immutalize() {
        if (this instanceof CopyOnWrite) {
            ((CopyOnWrite)this).copyOnWrite();
        }
        this._immutable = true;
    }
}