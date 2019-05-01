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
import com.tokera.ate.common.ImmutalizableHashSet;
import com.tokera.ate.common.UUIDTools;
import com.tokera.ate.dao.ObjId;
import com.tokera.ate.dao.msg.*;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.units.ClassName;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.validation.constraints.NotNull;
import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;
import javax.ws.rs.WebApplicationException;
import java.io.Serializable;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.Set;
import java.util.UUID;

/**
 * Represents key properties of a data message before its placed on the distributed commit log
 */
@YamlTag("msg.data.header")
public class MessageDataHeaderDto extends MessageBaseDto implements Serializable, CopyOnWrite {

    private static final long serialVersionUID = -8052777200722290736L;

    // When running in copy-on-write mode
    private transient @Nullable MessageDataHeader fb;

    // Fields that are serialized
    @JsonProperty
    @MonotonicNonNull
    private @DaoId UUID id;                                 // ID of the entity within this topic
    @JsonProperty
    @Nullable
    private @DaoId UUID version;                            // New version of this entity
    @JsonProperty
    @Nullable
    private @DaoId UUID parentId;                           // ID of the parent that the object is attached to
    @JsonProperty
    @Nullable
    private UUID previousVersion;                           // Previous version that this data object inherits from (used for data merging)
    @JsonProperty
    private ImmutalizableHashSet<UUID> merges = new ImmutalizableHashSet<>();             // List all of the versions that have been merged by this version
    @JsonProperty
    @MonotonicNonNull
    private Boolean inheritRead;                            // Should inherit read permissions from its parent
    @JsonProperty
    @MonotonicNonNull
    private Boolean inheritWrite;                           // Should inherit write permissions from its parent
    @JsonProperty
    @Nullable
    @Size(min=1)
    private @ClassName String payloadClazz;                 // Class of object held within this payload
    @JsonProperty
    @MonotonicNonNull
    @Size(min=43, max=43)
    @Pattern(regexp = "^(?:[A-Za-z0-9+\\/\\-_])*(?:[A-Za-z0-9+\\/\\-_]{2}==|[A-Za-z0-9+\\/\\-_]{3}=)?$")
    private @Hash String encryptKeyHash;          // Hash of the encryption key used for this payload
    @JsonProperty
    @NotNull
    private ImmutalizableHashSet<@Hash String> allowRead = new ImmutalizableHashSet<>();    // List of all the public key hashes roles that are allowed attach to this parent as a right to
    @JsonProperty
    @NotNull
    private ImmutalizableHashSet<@Hash String> allowWrite = new ImmutalizableHashSet<>();   // List of all the public key hashes roles that are allowed attach to this parent as a right to

    @JsonIgnore
    private transient boolean _immutable = false;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public MessageDataHeaderDto(){
    }

    public MessageDataHeaderDto(@DaoId UUID id, UUID version, @Nullable UUID previousVersion, @ClassName String payloadClazz) {
        this.id = id;
        this.version = version;
        this.payloadClazz = payloadClazz;
        this.previousVersion = previousVersion;
    }

    public MessageDataHeaderDto(MessageDataHeaderDto previousHeader) {
        this.id = previousHeader.getIdOrThrow();
        this.version = UUID.randomUUID();
        this.payloadClazz = previousHeader.payloadClazz;
        this.previousVersion = previousHeader.version;
        this.parentId = previousHeader.parentId;
        String encryptKeyHash = previousHeader.getEncryptKeyHash();
        if (encryptKeyHash != null) {
            this.encryptKeyHash = encryptKeyHash;
        }
        this.inheritRead = previousHeader.getInheritRead();
        this.inheritWrite = previousHeader.getInheritWrite();
        this.allowRead = new ImmutalizableHashSet<>(previousHeader.allowRead);
        this.allowWrite = new ImmutalizableHashSet<>(previousHeader.allowWrite);
    }

    public MessageDataHeaderDto(MessageDataHeader val)
    {
        this.fb = val;
    }

    public void setFlatBuffer(MessageDataHeader val) {
        assert this._immutable == false;
        this.fb = val;
    }

    @Override
    public void copyOnWrite()
    {
        MessageDataHeader lfb = fb;
        if (lfb == null) return;

        id = UUIDTools.convertUUID(lfb.id());
        version = UUIDTools.convertUUID(lfb.version());
        previousVersion = UUIDTools.convertUUIDOrNull(lfb.previousVersion());
        parentId = UUIDTools.convertUUIDOrNull(lfb.parentId());

        inheritRead = lfb.inheritRead();
        inheritWrite = lfb.inheritWrite();

        String payloadClazz = lfb.payloadClazz();
        if (payloadClazz != null) {
            this.payloadClazz = payloadClazz;
        }

        @Hash String encryptKeyHash = lfb.encryptKeyHash();
        if (encryptKeyHash != null) {
            this.encryptKeyHash = encryptKeyHash;
        }

        merges = new ImmutalizableHashSet<>();
        allowRead = new ImmutalizableHashSet<>();
        allowWrite = new ImmutalizableHashSet<>();
        for (int n = 0; n < lfb.mergesLength(); n++) {
            UUID parentVersion = UUIDTools.convertUUID(lfb.merges(n));
            merges.add(parentVersion);
        }
        for (int n = 0; n < lfb.allowReadLength(); n++) {
            String hash = lfb.allowRead(n);
            if (hash == null) continue;
            allowRead.add(hash);
        }
        for (int n = 0; n < lfb.allowWriteLength(); n++) {
            String hash = lfb.allowWrite(n);
            if (hash == null) continue;
            allowWrite.add(hash);
        }

        fb = null;
    }

    public @Nullable @DaoId UUID getId() {
        MessageDataHeader lfb = fb;
        if (lfb != null) {
            return UUIDTools.convertUUID(lfb.id());
        }
        return this.id;
    }

    public @DaoId UUID getIdOrThrow() {
        MessageDataHeader lfb = fb;
        if (lfb != null) {
            return UUIDTools.convertUUID(lfb.id());
        }
        @DaoId UUID ret = this.id;
        if (ret == null) throw new WebApplicationException("Message data header has no ID attached");
        return ret;
    }

    public void setId(@DaoId UUID id) {
        assert this._immutable == false;
        copyOnWrite();
        this.id = id;
    }

    public @Nullable UUID getVersion() {
        MessageDataHeader lfb = fb;
        if (lfb != null) {
            return UUIDTools.convertUUID(lfb.version());
        }
        return this.version;
    }

    public UUID getVersionOrThrow() {
        MessageDataHeader lfb = fb;
        if (lfb != null) {
            return UUIDTools.convertUUID(lfb.version());
        }
        UUID ret = this.version;
        if (ret == null) throw new WebApplicationException("Message data header has no version attached");
        return ret;
    }

    public void setPreviousVersion(UUID previousVersion) {
        assert this._immutable == false;
        copyOnWrite();
        this.previousVersion = previousVersion;
    }

    public @Nullable UUID getPreviousVersion() {
        MessageDataHeader lfb = fb;
        if (lfb != null) {
            return UUIDTools.convertUUIDOrNull(lfb.previousVersion());
        }
        return this.previousVersion;
    }

    public void setVersion(UUID version) {
        assert this._immutable == false;
        copyOnWrite();
        this.version = version;
    }

    public void newVersion() {
        copyOnWrite();
        this.previousVersion = this.version;
        this.version = UUID.randomUUID();
    }

    public ImmutalizableHashSet<UUID> getMerges() {
        MessageDataHeader lfb = fb;
        if (lfb != null)
        {
            ImmutalizableHashSet<UUID> ret = new ImmutalizableHashSet<>();
            for (int n = 0; n < lfb.mergesLength(); n++) {
                UUID parentVersion = UUIDTools.convertUUID(lfb.merges(n));
                ret.add(parentVersion);
            }
            return ret;
        }
        return this.merges;
    }

    public void setMerges(Set<UUID> mergeVersions) {
        assert this._immutable == false;
        copyOnWrite();
        this.merges = new ImmutalizableHashSet<>(mergeVersions);
    }

    public @Nullable @DaoId UUID getParentId() {
        MessageDataHeader lfb = fb;
        if (lfb != null) {
            return UUIDTools.convertUUIDOrNull(lfb.parentId());
        }
        return parentId;
    }

    public void setParentId(@DaoId UUID parentId) {
        assert this._immutable == false;
        copyOnWrite();
        this.parentId = parentId;
    }

    public @Nullable @ClassName String getPayloadClazz() {
        MessageDataHeader lfb = fb;
        if (lfb != null) {
            @ClassName String v = lfb.payloadClazz();
            if (v != null) return v;
        }
        return this.payloadClazz;
    }

    public @ClassName String getPayloadClazzOrThrow() {
        MessageDataHeader lfb = fb;
        if (lfb != null) {
            @ClassName String v = lfb.payloadClazz();
            if (v != null) return v;
        }
        String ret = this.payloadClazz;
        if (ret == null) throw new WebApplicationException("Message data header has no payload clazz attached.");
        return ret;
    }

    public void setPayloadClazz(@ClassName String payloadClazz) {
        assert this._immutable == false;
        copyOnWrite();
        this.payloadClazz = payloadClazz;
    }

    public @Nullable @Hash String getEncryptKeyHash() {
        MessageDataHeader lfb = fb;
        if (lfb != null) {
            @Hash String v = lfb.encryptKeyHash();
            if (v != null) return v;
        }
        return encryptKeyHash;
    }

    public void setEncryptKeyHash(@Hash String encryptKeyHash) {
        assert this._immutable == false;
        copyOnWrite();
        this.encryptKeyHash = encryptKeyHash;
    }

    public Set<@Hash String> getAllowRead() {
        MessageDataHeader lfb = fb;
        if (lfb != null)
        {
            HashSet<@Hash String> ret = new HashSet<>();
            for (int n = 0; n < lfb.allowReadLength(); n++) {
                @Hash String v = lfb.allowRead(n);
                if (v == null) continue;
                ret.add(v);
            }
            return ret;
        }
        return allowRead;
    }

    public void setAllowRead(Set<@Hash String> allowRead) {
        assert this._immutable == false;
        copyOnWrite();
        this.allowRead = new ImmutalizableHashSet<>(allowRead);
    }

    public Set<@Hash String> getAllowWrite() {
        MessageDataHeader lfb = fb;
        if (lfb != null)
        {
            HashSet<@Hash String> ret = new HashSet<>();
            for (int n = 0; n < lfb.allowWriteLength(); n++) {
                @Hash String v = lfb.allowWrite(n);
                if (v == null) continue;
                ret.add(v);
            }
            return ret;
        }
        return allowWrite;
    }

    public void setAllowWrite(Set<@Hash String> allowWrite) {
        assert this._immutable == false;
        copyOnWrite();
        this.allowWrite = new ImmutalizableHashSet<>(allowWrite);
    }

    public boolean getInheritRead() {
        MessageDataHeader lfb = fb;
        if (lfb != null) {
            return lfb.inheritRead();
        }
        Boolean ret = this.inheritRead;
        if (ret == null) return true;
        return ret.booleanValue();
    }

    public void setInheritRead(boolean inheritRead) {
        assert this._immutable == false;
        copyOnWrite();
        this.inheritRead = inheritRead;
    }

    public boolean getInheritWrite() {
        MessageDataHeader lfb = fb;
        if (lfb != null) {
            return lfb.inheritWrite();
        }
        Boolean ret = this.inheritWrite;
        if (ret == null) return true;
        return ret.booleanValue();
    }

    public void setInheritWrite(boolean inheritWrite) {
        assert this._immutable == false;
        copyOnWrite();
        this.inheritWrite = inheritWrite;
    }

    @Override
    public int flatBuffer(FlatBufferBuilder fbb)
    {
        Set<UUID> theMergeVersions = this.getMerges();
        ArrayList<UUID> mergeVersions = new ArrayList<>();
        if (theMergeVersions.size() > 0) {
            for (UUID v : theMergeVersions) {
                mergeVersions.add(v);
            }
        }

        Set<String> theAllowReads = this.getAllowRead();
        ArrayList<Integer> reads = new ArrayList<>();
        if (theAllowReads.size() > 0) {
            for (String s : theAllowReads) {
                reads.add(fbb.createString(s));
            }
        }

        Set<String> theAllowWrites = this.getAllowWrite();
        ArrayList<Integer> writes = new ArrayList<>();
        if (theAllowWrites.size() > 0) {
            for (String s : theAllowWrites) {
                writes.add(fbb.createString(s));
            }
        }

        // Add all the other other fields for the header
        String strPayloadClazz = this.getPayloadClazzOrThrow();
        int offsetPayloadClazz = fbb.createString(strPayloadClazz);

        String strEncryptKey = this.getEncryptKeyHash();
        int offsetEncryptKeyHash = -1;
        if (strEncryptKey != null) {
            offsetEncryptKeyHash = fbb.createString(strEncryptKey);
        }

        int offsetMergeVersions = -1;
        if (mergeVersions.size() > 0) {
            MessageDataHeader.startMergesVector(fbb, mergeVersions.size());
            for (int i = mergeVersions.size() - 1; i >= 0; i--) {
                UUID v = mergeVersions.get(i);
                ObjId.createObjId(fbb,v.getLeastSignificantBits(), v.getMostSignificantBits());
            }
            offsetMergeVersions = fbb.endVector();
        }

        int offsetAllowRead = MessageDataHeader.createAllowReadVector(fbb, reads.stream().mapToInt(i -> i).toArray());

        int offsetAllowWrite = MessageDataHeader.createAllowWriteVector(fbb, writes.stream().mapToInt(i -> i).toArray());

        MessageDataHeader.startMessageDataHeader(fbb);
        MessageDataHeader.addId(fbb, ObjId.createObjId(fbb, this.getIdOrThrow().getLeastSignificantBits(), this.getIdOrThrow().getMostSignificantBits()));
        MessageDataHeader.addVersion(fbb, ObjId.createObjId(fbb, this.getVersionOrThrow().getLeastSignificantBits(), this.getVersionOrThrow().getMostSignificantBits()));
        UUID previousVersion = this.getPreviousVersion();
        if (previousVersion != null) {
            MessageDataHeader.addPreviousVersion(fbb, ObjId.createObjId(fbb, previousVersion.getLeastSignificantBits(), previousVersion.getMostSignificantBits()));
        }

        @DaoId UUID parentId = this.getParentId();
        if (parentId != null) {
            MessageDataHeader.addParentId(fbb, ObjId.createObjId(fbb, parentId.getLeastSignificantBits(), parentId.getMostSignificantBits()));
        }

        if (offsetPayloadClazz >= 0) MessageDataHeader.addPayloadClazz(fbb, offsetPayloadClazz);
        if (offsetEncryptKeyHash >= 0) MessageDataHeader.addEncryptKeyHash(fbb, offsetEncryptKeyHash);
        MessageDataHeader.addInheritRead(fbb, this.getInheritRead());
        MessageDataHeader.addInheritWrite(fbb, this.getInheritWrite());
        if (offsetMergeVersions >= 0) MessageDataHeader.addMerges(fbb, offsetMergeVersions);
        if (offsetAllowRead >= 0) MessageDataHeader.addAllowRead(fbb, offsetAllowRead);
        if (offsetAllowWrite >= 0) MessageDataHeader.addAllowWrite(fbb, offsetAllowWrite);
        return MessageDataHeader.endMessageDataHeader(fbb);
    }

    public MessageDataHeader createFlatBuffer()
    {
        FlatBufferBuilder fbb = new FlatBufferBuilder();
        fbb.finish(flatBuffer(fbb));
        return MessageDataHeader.getRootAsMessageDataHeader(fbb.dataBuffer());
    }

    public void immutalize() {
        this._immutable = true;
        this.merges.immutalize();
        this.allowRead.immutalize();
        this.allowWrite.immutalize();
    }
}