/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto.fs;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.units.Alias;

import javax.validation.constraints.NotNull;
import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;

/**
 * Represents a file
 */
@YamlTag("dto.fs.file")
public class FsFileDto {

    @JsonProperty
    @NotNull
    @Size(min=1, max=64)
    @Pattern(regexp = "^[a-zA-Z0-9_\\-\\:\\@\\.]+$")
    private @Alias String name;
    @JsonProperty
    @NotNull
    private Boolean execute = false;
    @JsonProperty
    @NotNull
    private Boolean writeable = false;
    @JsonProperty
    @NotNull
    private Boolean cacheResults = false;
    @JsonProperty
    @NotNull
    private Boolean partialPut = false;
    @JsonProperty
    @NotNull
    private Boolean passthrough = false;
    @JsonProperty
    @NotNull
    private Boolean createPass = false;
    @JsonProperty
    @NotNull
    private Boolean ethereal = false;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public FsFileDto() {
    }

    public FsFileDto(String name) {
        this.name = name;
    }
    
    public FsFileDto(String name, boolean writeable) {
        this.name = name;
        this.writeable = writeable;
    }

    public @Alias String getName() {
        return this.name;
    }

    public FsFileDto setName(@Alias String value) {
        this.name = value;
        return this;
    }

    public Boolean getExecute() {
        return this.execute;
    }
    
    public FsFileDto setExecute(Boolean value) {
        this.execute = value;
        return this;
    }
    
    public Boolean getWriteable() {
        return this.writeable;
    }
    
    public FsFileDto setWriteable(Boolean value) {
        this.writeable = value;
        return this;
    }
    
    public Boolean getPartialPut() {
        return partialPut;
    }
    
    public FsFileDto setPartialPut(Boolean partialPut) {
        this.partialPut = partialPut;
        return this;
    }
    
    public Boolean getPassthrough() {
        return passthrough;
    }
    
    public FsFileDto setPassthrough(Boolean passthrough) {
        this.passthrough = passthrough;
        return this;
    }

    public Boolean getCreatePass() {
        return createPass;
    }

    public FsFileDto setCreatePass(Boolean createPass) {
        this.createPass = createPass;
        return this;
    }

    public Boolean getCacheResults() {
        return cacheResults;
    }

    public void setCacheResults(Boolean cacheResults) {
        this.cacheResults = cacheResults;
    }
    
    public Boolean getEthereal() {
        return ethereal;
    }
    
    public FsFileDto setEthereal(Boolean ethereal) {
        this.ethereal = ethereal;
        return this;
    }
}