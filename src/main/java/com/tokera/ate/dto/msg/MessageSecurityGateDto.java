package com.tokera.ate.dto.msg;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.google.flatbuffers.FlatBufferBuilder;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.common.CopyOnWrite;
import com.tokera.ate.common.Immutalizable;
import com.tokera.ate.dao.msg.MessageSecurityGate;
import com.tokera.ate.units.Hash;
import com.tokera.ate.units.Secret;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;
import javax.ws.rs.WebApplicationException;
import java.io.Serializable;
import java.nio.ByteBuffer;
import java.util.Objects;

@Dependent
@YamlTag("msg.gate")
public class MessageSecurityGateDto implements Serializable, CopyOnWrite, Immutalizable {
    private static final long serialVersionUID = -4924053477617023297L;

    private transient @Nullable MessageSecurityGate fb;

    @JsonProperty
    @MonotonicNonNull
    @Size(min=43, max=43)
    @Pattern(regexp = "^(?:[A-Za-z0-9+\\/\\-_])*(?:[A-Za-z0-9+\\/\\-_]{2}==|[A-Za-z0-9+\\/\\-_]{3}=)?$")
    private @Hash String publicKeyHash;
    @JsonProperty
    @MonotonicNonNull
    @Size(min = 2)
    private @Secret String encryptedText;

    @JsonIgnore
    private transient boolean _immutable = false;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public MessageSecurityGateDto(){
    }

    public MessageSecurityGateDto(@Hash String publicKeyHash, @Secret String encryptedText) {
        this.publicKeyHash = publicKeyHash;
        this.encryptedText = encryptedText;
    }

    public MessageSecurityGateDto(@Hash String publicKeyHash, @Secret byte[] encryptedTextBytes) {
        this.publicKeyHash = publicKeyHash;
        this.encryptedText = Base64.encodeBase64URLSafeString(encryptedTextBytes);
    }

    public MessageSecurityGateDto(MessageSecurityGate val) {
        fb = val;
    }

    @Override
    public void copyOnWrite() {
        MessageSecurityGate lfb = fb;
        if (lfb == null) return;

        String pubKeyHash = lfb.publicKeyHash();
        if (pubKeyHash != null) {
            this.publicKeyHash = pubKeyHash;
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
        MessageSecurityGate lfb = fb;
        if (lfb != null) {
            @Hash String v = lfb.publicKeyHash();
            if (v != null) return v;
        }

        @Hash String ret = this.publicKeyHash;
        if (ret == null) throw new WebApplicationException("MessageSecurityGate has no public key hash.");
        return ret;
    }

    /**
     * @param publicKeyHash the publicKeyHash to set
     */
    public void setPublicKeyHash(@Hash String publicKeyHash) {
        assert this._immutable == false;
        copyOnWrite();
        this.publicKeyHash = publicKeyHash;
    }

    public @Secret String getEncryptedText() {
        MessageSecurityGate lfb = fb;
        if (lfb != null) {
            ByteBuffer bb = lfb.encryptedTextAsByteBuffer();
            if (bb != null) {
                byte[] bytes = new byte[bb.remaining()];
                bb.get(bytes);
                return Base64.encodeBase64URLSafeString(bytes);
            }
        }

        @Secret String ret = this.encryptedText;
        if (ret == null) throw new WebApplicationException("MessageSecurityGate has no encrypted text attached.");
        return ret;
    }

    public void setEncryptedText(@Secret String val) {
        assert this._immutable == false;
        copyOnWrite();
        this.encryptedText = val;
    }

    /**
     * @return the encryptedText
     */
    public @Secret byte[] getEncryptedTextBytes() {
        MessageSecurityGate lfb = fb;
        if (lfb != null) {
            ByteBuffer bb = lfb.encryptedTextAsByteBuffer();
            if (bb == null) throw new WebApplicationException("MessageSecurityGate has no encrypt text attached.");

            byte[] bytes = new byte[bb.remaining()];
            bb.get(bytes);
            return bytes;
        }

        @Secret String ret = this.encryptedText;
        if (ret == null) throw new WebApplicationException("MessageSecurityGate has no encrypt text attached.");
        return Base64.decodeBase64(ret);
    }

    /**
     * @param encryptedTextBytes the encryptedText to set
     */
    public void setEncryptedTextBytes(@Secret byte[] encryptedTextBytes) {
        copyOnWrite();
        this.encryptedText = Base64.encodeBase64URLSafeString(encryptedTextBytes);
    }

    public int flatBuffer(FlatBufferBuilder fbb)
    {
        String strPublicKeyHash = this.getPublicKeyHash();
        int offsetPublicKeyHash = fbb.createString(strPublicKeyHash);

        byte[] bytesEncryptedText = this.getEncryptedTextBytes();
        int offsetEncryptedText = MessageSecurityGate.createEncryptedTextVector(fbb, bytesEncryptedText);

        MessageSecurityGate.startMessageSecurityGate(fbb);
        if (offsetPublicKeyHash >= 0) MessageSecurityGate.addPublicKeyHash(fbb, offsetPublicKeyHash);
        if (offsetEncryptedText >= 0) MessageSecurityGate.addEncryptedText(fbb, offsetEncryptedText);
        return MessageSecurityGate.endMessageSecurityGate(fbb);
    }

    public MessageSecurityGate createFlatBuffer()
    {
        FlatBufferBuilder fbb = new FlatBufferBuilder();
        fbb.finish(flatBuffer(fbb));
        return MessageSecurityGate.getRootAsMessageSecurityGate(fbb.dataBuffer());
    }

    @Override
    public void immutalize() {
        if (this instanceof CopyOnWrite) {
            ((CopyOnWrite)this).copyOnWrite();
        }
        this._immutable = true;
    }

    @Override
    public boolean equals(@Nullable Object o)
    {
        if (o == null) return false;
        if (getClass() != o.getClass()) return false;

        MessageSecurityGateDto that = (MessageSecurityGateDto) o;

        if (Objects.equals(this.publicKeyHash, that.publicKeyHash) == false) return false;

        return true;
    }

    @Override
    public int hashCode()
    {
        int result = (int)0;
        if (this.publicKeyHash != null) result += this.publicKeyHash.hashCode();
        return result;
    }
}
