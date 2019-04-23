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
import java.util.ArrayList;
import java.util.List;

/**
 * Represents a folder of files and sub directories
 *
 * @author root
 */
@YamlTag("dto.fs.folder")
public class FsFolderDto {

    @JsonProperty
    @NotNull
    @Size(min=1, max=64)
    @Pattern(regexp = "^[a-zA-Z0-9_\\-\\:\\@\\.]+$")
    private @Alias String name;
    @JsonProperty
    @NotNull
    private Boolean passthrough = false;
    @JsonProperty
    @NotNull
    private Boolean createPass = false;
    @JsonProperty
    @NotNull
    private Boolean cacheResults = false;
    @JsonProperty
    @NotNull
    private Boolean writeable = false;
    @JsonProperty
    @NotNull
    private List<FsFolderDto> subFolders = new ArrayList<>();
    @JsonProperty
    @NotNull
    private List<FsFileDto> files = new ArrayList<>();
    @JsonProperty
    @NotNull
    private List<FsSymbolicDto> symbolics = new ArrayList<>();
    @JsonProperty
    @NotNull
    private List<FsMountDto> mounts = new ArrayList<>();

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public FsFolderDto() {
    }

    public FsFolderDto(@Alias String name) {
        this.name = name;
    }
    
    public FsFolderDto(@Alias String name, boolean write) {
        this.name = name;
        this.writeable = write;
    }

    public @Alias String getName() {
        return this.name;
    }

    public FsFolderDto setName(@Alias String value) {
        this.name = value;
        return this;
    }

    public List<FsFolderDto> getSubFolders() {
        return this.subFolders;
    }

    public FsFolderDto setSubFolders(List<FsFolderDto> value) {
        this.subFolders = value;
        return this;
    }

    public List<FsFileDto> getFiles() {
        return this.files;
    }

    public FsFolderDto setFiles(List<FsFileDto> value) {
        this.files = value;
        return this;
    }

    public List<FsSymbolicDto> getSymbolics() {
        return this.symbolics;
    }

    public FsFolderDto setSymbolics(List<FsSymbolicDto> value) {
        this.symbolics = value;
        return this;
    }
    
    public List<FsMountDto> getMounts() {
        return mounts;
    }
    
    public FsFolderDto setMounts(List<FsMountDto> mounts) {
        this.mounts = mounts;
        return this;
    }
    
    public Boolean getPassthrough() {
        return passthrough;
    }
    
    public FsFolderDto setPassthrough(Boolean passthrough) {
        this.passthrough = passthrough;
        return this;
    }

    public Boolean getCreatePass() {
        return createPass;
    }

    public FsFolderDto setCreatePass(Boolean createPass) {
        this.createPass = createPass;
        return this;
    }

    public Boolean getCacheResults() {
        return cacheResults;
    }

    public void setCacheResults(Boolean cacheResults) {
        this.cacheResults = cacheResults;
    }
    
    public Boolean getWriteable() {
        return writeable;
    }

    public void setWriteable(Boolean writeable) {
        this.writeable = writeable;
    }
}
