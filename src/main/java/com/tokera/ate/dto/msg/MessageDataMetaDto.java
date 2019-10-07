/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto.msg;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.Mergable;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.common.CopyOnWrite;

import javax.enterprise.context.Dependent;
import javax.validation.constraints.NotNull;
import java.io.Serializable;
import java.util.UUID;

/**
 * Represents a bundle of the data and its on commit log metadata
 */
@Mergable
@Dependent
@YamlTag("msg.data.meta")
public class MessageDataMetaDto implements Serializable {

    private static final long serialVersionUID = 234340464367516609L;

    @JsonProperty
    @NotNull
    private MessageDataDto data;
    @JsonProperty
    @NotNull
    private MessageMetaDto meta;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public MessageDataMetaDto() {
    }

    public MessageDataMetaDto(MessageDataDto data, MessageMetaDto meta) {
        this.data = data;
        this.meta = meta;
    }

    public MessageDataDto getData() { return this.data; }

    public void setData(MessageDataDto val) { this.data = val; }

    public MessageMetaDto getMeta() { return this.meta; }

    public void setMeta(MessageMetaDto val) { this.meta = val; }

    public UUID version() {
        return this.data.getHeader().getVersionOrThrow();
    }

    public void immutalize() {
        if (this instanceof CopyOnWrite) {
            ((CopyOnWrite)this).copyOnWrite();
        }
        this.data.immutalize();
        this.meta.immutalize();
    }

    @JsonIgnore
    public MessageDataHeaderDto getHeader() {
        return getData().getHeader();
    }

    @JsonIgnore
    public UUID getVersionOrThrow() {
        return getHeader().getVersionOrThrow();
    }

    @JsonIgnore
    public boolean hasPayload() {
        if (this.data == null) return false;
        return this.data.hasPayload();
    }
}