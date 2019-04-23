package com.tokera.ate.dto.msg;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.google.flatbuffers.FlatBufferBuilder;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.common.CopyOnWrite;
import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.dao.msg.MessageEncryptText;
import com.tokera.ate.dao.msg.MessageType;

import java.io.Serializable;
import java.nio.ByteBuffer;
import java.util.Objects;

import com.tokera.ate.units.Hash;
import com.tokera.ate.units.Secret;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;
import javax.ws.rs.WebApplicationException;

/**
 * Represents an AES encrypted piece of text thats distributed on the commit log and associated with a particular
 * set of publickey and text based hashes used before the encryption took place
 */
@YamlTag("msg.encrypt.text")
public class MessageEncryptTextDto extends MessageBaseDto implements Serializable, CopyOnWrite {

    private static final long serialVersionUID = -5434346989770912304L;

    @Nullable
    private transient MessageEncryptText fb;

    @JsonProperty
    @MonotonicNonNull
    @Size(min=43, max=43)
    @Pattern(regexp = "^(?:[A-Za-z0-9+\\/\\-_])*(?:[A-Za-z0-9+\\/\\-_]{2}==|[A-Za-z0-9+\\/\\-_]{3}=)?$")
    private @Hash String publicKeyHash;
    @JsonProperty
    @MonotonicNonNull
    @Size(min=43, max=43)
    @Pattern(regexp = "^(?:[A-Za-z0-9+\\/\\-_])*(?:[A-Za-z0-9+\\/\\-_]{2}==|[A-Za-z0-9+\\/\\-_]{3}=)?$")
    private @Hash String textHash;
    @JsonProperty
    @MonotonicNonNull
    @Size(min = 2)
    private @Secret String encryptedText;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public MessageEncryptTextDto() {
    }

    public MessageEncryptTextDto(@Hash String publicKeyHash, @Hash String textHash, @Secret String encryptedText) {
        this.publicKeyHash = publicKeyHash;
        this.textHash = textHash;
        this.encryptedText = encryptedText;
    }

    public MessageEncryptTextDto(@Hash String publicKeyHash, @Hash String textHash, @Secret byte[] encryptedTextBytes) {
        this.publicKeyHash = publicKeyHash;
        this.textHash = textHash;
        this.encryptedText = Base64.encodeBase64URLSafeString(encryptedTextBytes);
    }
    
    public MessageEncryptTextDto(MessageEncryptText val) {
        fb = val;
    }
    
    public MessageEncryptTextDto(MessageBase val) {
        if (val.msgType() == MessageType.MessageEncryptText) {
            fb = (MessageEncryptText)val.msg(new MessageEncryptText());
        } else {
            throw new WebApplicationException("Invalidate message type [expected=MessageEncryptText, actual=" + val.msgType() + "]");
        }
    }

    @Override
    public void copyOnWrite() {
        MessageEncryptText lfb = fb;
        if (lfb == null) return;

        String pubKeyHash = lfb.publicKeyHash();
        if (pubKeyHash != null) {
            this.publicKeyHash = pubKeyHash;
        }

        String textHash = lfb.textHash();
        if (textHash != null) {
            this.textHash = textHash;
        }
        
        if (lfb.encryptedTextLength() > 0) {
            ByteBuffer bb = lfb.encryptedTextAsByteBuffer();
            if (bb != null) {
                byte[] bytes = new byte[bb.remaining()];
                bb.get(bytes);
                this.encryptedText = Base64.encodeBase64URLSafeString(bytes);
            }
        }

        fb = null;
    }
    
    /**
     * @return the publicKeyHash
     */
    public @Hash String getPublicKeyHash() {
        MessageEncryptText lfb = fb;
        if (lfb != null) {
            @Hash String v = lfb.publicKeyHash();
            if (v != null) return v;
        }

        @Hash String ret = this.publicKeyHash;
        if (ret == null) throw new WebApplicationException("MessageEncryptText has no public key hash.");
        return ret;
    }

    /**
     * @param publicKeyHash the publicKeyHash to set
     */
    public void setPublicKeyHash(@Hash String publicKeyHash) {
        copyOnWrite();
        this.publicKeyHash = publicKeyHash;
    }

    /**
     * @return the textHash
     */
    public @Hash String getTextHash() {
        MessageEncryptText lfb = fb;
        if (lfb != null) {
            @Hash String v = lfb.textHash();
            if (v != null) return v;
        }

        @Hash String ret = this.textHash;
        if (ret == null) throw new WebApplicationException("MessageEncryptText has no text hash attached.");
        return ret;
    }

    /**
     * @param textHash the textHash to set
     */
    public void setTextHash(@Hash String textHash) {
        copyOnWrite();
        this.textHash = textHash;
    }
    
    public @Secret String getEncryptedText() {
        MessageEncryptText lfb = fb;
        if (lfb != null) {
            ByteBuffer bb = lfb.encryptedTextAsByteBuffer();
            if (bb != null) {
                byte[] bytes = new byte[bb.remaining()];
                bb.get(bytes);
                return Base64.encodeBase64URLSafeString(bytes);
            }
        }

        @Secret String ret = this.encryptedText;
        if (ret == null) throw new WebApplicationException("MessageEncryptText has no encrypted text attached.");
        return ret;
    }
    
    public void setEncryptedText(@Secret String val) {
        copyOnWrite();
        this.encryptedText = val;
    }

    /**
     * @return the encryptedText
     */
    public @Secret byte[] getEncryptedTextBytes() {
        MessageEncryptText lfb = fb;
        if (lfb != null) {
            ByteBuffer bb = lfb.encryptedTextAsByteBuffer();
            if (bb == null) throw new WebApplicationException("MessageEncryptText has no encrypt text attached.");

            byte[] bytes = new byte[bb.remaining()];
            bb.get(bytes);
            return bytes;
        }

        @Secret String ret = this.encryptedText;
        if (ret == null) throw new WebApplicationException("MessageEncryptText has no encrypt text attached.");
        return Base64.decodeBase64(ret);
    }

    /**
     * @param encryptedTextBytes the encryptedText to set
     */
    public void setEncryptedTextBytes(@Secret byte[] encryptedTextBytes) {
        copyOnWrite();
        this.encryptedText = Base64.encodeBase64URLSafeString(encryptedTextBytes);
    }
    
    @Override
    public int flatBuffer(FlatBufferBuilder fbb)
    {
        String strPublicKeyHash = this.getPublicKeyHash();
        int offsetPublicKeyHash = fbb.createString(strPublicKeyHash);

        String strTextHash = this.getTextHash();
        int offsetTextHash = fbb.createString(strTextHash);
        
        byte[] bytesEncryptedText = this.getEncryptedTextBytes();
        int offsetEncryptedText = MessageEncryptText.createEncryptedTextVector(fbb, bytesEncryptedText);

        MessageEncryptText.startMessageEncryptText(fbb);
        if (offsetPublicKeyHash >= 0) MessageEncryptText.addPublicKeyHash(fbb, offsetPublicKeyHash);
        if (offsetTextHash >= 0) MessageEncryptText.addTextHash(fbb, offsetTextHash);
        if (offsetEncryptedText >= 0) MessageEncryptText.addEncryptedText(fbb, offsetEncryptedText);
        return MessageEncryptText.endMessageEncryptText(fbb);
    }
    
    public MessageEncryptText createFlatBuffer()
    {
        FlatBufferBuilder fbb = new FlatBufferBuilder();
        fbb.finish(flatBuffer(fbb));
        return MessageEncryptText.getRootAsMessageEncryptText(fbb.dataBuffer());
    }

    @Override
    public boolean equals(@Nullable Object o)
    {
        if (o == null) return false;
        if (getClass() != o.getClass()) return false;

        MessageEncryptTextDto that = (MessageEncryptTextDto) o;

        if (Objects.equals(this.textHash, that.textHash) == false) return false;
        if (Objects.equals(this.publicKeyHash, that.publicKeyHash) == false) return false;

        return true;
    }

    @Override
    public int hashCode()
    {
        int result = (int)0;
        if (this.textHash != null) result += this.textHash.hashCode();
        if (this.publicKeyHash != null) result += this.publicKeyHash.hashCode();
        return result;
    }
}
