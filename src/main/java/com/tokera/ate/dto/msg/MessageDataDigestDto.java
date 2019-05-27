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
import com.tokera.ate.dao.msg.*;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.units.Hash;
import com.tokera.ate.units.PlainText;
import com.tokera.ate.units.Salt;
import com.tokera.ate.units.Signature;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;
import javax.ws.rs.WebApplicationException;
import java.io.Serializable;
import java.nio.ByteBuffer;

/**
 * Represents the digest of a data payload signed by an authorized user
 */
@YamlTag("msg.data.digest")
public class MessageDataDigestDto extends MessageBaseDto implements Serializable, CopyOnWrite {

    private static final long serialVersionUID = 3992438221645570455L;

    // When running in copy-on-write mode
    private transient @Nullable MessageDataDigest fb;
    
    // Fields that are serialized
    @JsonProperty
    @MonotonicNonNull
    private @Salt String seed;                // Seed added to the digest calculation
    @JsonProperty
    @MonotonicNonNull
    private @Signature String signature1;           // digitally signed digest of the payload _after_ it was encrypted
    @JsonProperty
    @MonotonicNonNull
    private @Signature String signature2;           // digitally signed digest of the payload _after_ it was encrypted
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
            @Signature String signature1,
            @Signature String signature2,
            @Hash String digest,
            @Hash String publicKeyHash)
    {
        this.seed = seed;
        this.signature1 = signature1;
        this.signature2 = signature2;
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
        if (lfb.signature1Length() > 0) {
            ByteBuffer bb = lfb.signature1AsByteBuffer();
            if (bb != null) {
                String v = ByteBufferTools.toBase64(bb);
                if (v != null) signature1 = v;
            }
        }
        if (lfb.signature2Length() > 0) {
            ByteBuffer bb = lfb.signature2AsByteBuffer();
            if (bb != null) {
                String v = ByteBufferTools.toBase64(bb);
                if (v != null) signature2 = v;
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
    
    public @Signature String getSignature1() {
        MessageDataDigest lfb = fb;
        if (lfb != null) {
            if (lfb.signature1Length() > 0) {
                ByteBuffer bb = lfb.signature1AsByteBuffer();
                if (bb != null) {
                    @Signature String v = ByteBufferTools.toBase64(bb);
                    if (v != null) return v;
                }
            }
        }
        @Signature String ret = this.signature1;
        if (ret == null) throw new WebApplicationException("MessageDataDigest has no signature bytes attached.");
        return ret;
    }

    public @Signature String getSignature2() {
        MessageDataDigest lfb = fb;
        if (lfb != null) {
            if (lfb.signature2Length() > 0) {
                ByteBuffer bb = lfb.signature2AsByteBuffer();
                if (bb != null) {
                    @Signature String v = ByteBufferTools.toBase64(bb);
                    if (v != null) return v;
                }
            }
        }
        @Signature String ret = this.signature2;
        if (ret == null) throw new WebApplicationException("MessageDataDigest has no signature bytes attached.");
        return ret;
    }
    
    public @Signature byte[] getSignatureBytes1() {
        MessageDataDigest lfb = fb;
        if (lfb != null) {
            if (lfb.signature1Length() > 0) {
                ByteBuffer bb = lfb.signature1AsByteBuffer();
                if (bb != null) {
                    byte[] arr = new byte[bb.remaining()];
                    bb.get(arr);
                    return arr;
                }
            }
        }
        @Signature String ret = this.signature1;
        if (ret == null) throw new WebApplicationException("MessageDataDigest has no signature bytes attached.");
        return Base64.decodeBase64(ret);
    }

    public void setSignature1(@Signature String signature) {
        assert this._immutable == false;
        copyOnWrite();
        this.signature1 = signature;
    }

    public @Signature byte[] getSignatureBytes2() {
        MessageDataDigest lfb = fb;
        if (lfb != null) {
            if (lfb.signature2Length() > 0) {
                ByteBuffer bb = lfb.signature2AsByteBuffer();
                if (bb != null) {
                    byte[] arr = new byte[bb.remaining()];
                    bb.get(arr);
                    return arr;
                }
            }
        }
        @Signature String ret = this.signature2;
        if (ret == null) throw new WebApplicationException("MessageDataDigest has no signature bytes attached.");
        return Base64.decodeBase64(ret);
    }

    public void setSignature(@Signature String signature) {
        assert this._immutable == false;
        copyOnWrite();
        this.signature2 = signature;
    }

    public @Hash String getPublicKeyHash() {
        MessageDataDigest lfb = fb;
        if (lfb != null) {
            @Hash String v = lfb.publicKeyHash();
            if (v != null) return v;
        }
        String ret = this.publicKeyHash;
        if (ret == null) throw new WebApplicationException("MessageDataDigest has no public key hash attached.");
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

    public void setSeed(@Salt String seed) {
        assert this._immutable == false;
        copyOnWrite();
        this.seed = seed;
    }
    
    public @PlainText byte[] getSeedBytes() {
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
        if (ret == null) throw new WebApplicationException("MessageDataDigest has no seed bytes attached.");
        return Base64.decodeBase64(ret);
    }
    
    public @Hash String getDigest() {
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

        @Hash String ret = this.digest;
        if (ret == null) throw new WebApplicationException("MessageDataDigest has no digest bytes attached.");
        return ret;
    }
    
    public @Hash byte[] getDigestBytes() {
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
        if (ret == null) throw new WebApplicationException("MessageDataDigest has no digest bytes attached.");
        return Base64.decodeBase64(ret);
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
        int offsetSignature1 = -1;
        int offsetSignature2 = -1;
        int offsetDigest = -1;
        int offsetPublicKeyHash = -1;
        
        String seedStr = this.getSeed();
        if (seedStr != null && seedStr.length() > 0) {
            offsetSeed = MessageDataDigest.createSeedVector(fbb, Base64.decodeBase64(seedStr));
        }
        
        String sigStr1 = this.getSignature1();
        if (sigStr1.length() > 0) {
            offsetSignature1 = MessageDataDigest.createSignature1Vector(fbb, Base64.decodeBase64(sigStr1));
        }

        String sigStr2 = this.getSignature2();
        if (sigStr2.length() > 0) {
            offsetSignature2 = MessageDataDigest.createSignature2Vector(fbb, Base64.decodeBase64(sigStr2));
        }
        
        String digestStr = this.getDigest();
        if (digestStr.length() > 0) {
            offsetDigest = MessageDataDigest.createDigestVector(fbb, Base64.decodeBase64(this.getDigest()));
        }
        
        String publicKeyHashStr = this.getPublicKeyHash();
        if (publicKeyHashStr.length() > 0) {
            offsetPublicKeyHash = fbb.createString(publicKeyHashStr);
        }
        
        MessageDataDigest.startMessageDataDigest(fbb);
        if (offsetSeed >= 0) MessageDataDigest.addSeed(fbb, offsetSeed);
        if (offsetSignature1 >= 0) MessageDataDigest.addSignature1(fbb, offsetSignature1);
        if (offsetSignature2 >= 0) MessageDataDigest.addSignature2(fbb, offsetSignature2);
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
        this._immutable = true;
    }
}