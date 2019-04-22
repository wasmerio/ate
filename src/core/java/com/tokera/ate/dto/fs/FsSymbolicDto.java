/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto.fs;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.Filepath;

import javax.validation.constraints.NotNull;
import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;

/**
 * Represents a folder of files and sub directories
 *
 * @author root
 */
@YamlTag("dto.fs.symbolic")
public class FsSymbolicDto {

    @JsonProperty
    @NotNull
    @Size(min=1, max=64)
    @Pattern(regexp = "^[a-zA-Z0-9_\\-\\:\\@\\.]+$")
    private @Alias String name;
    @JsonProperty
    @NotNull
    @Pattern(regexp = "^[\\w,\\s-\\.\\/)]+$")
    private @Filepath String path;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public FsSymbolicDto() {
    }

    public FsSymbolicDto(@Alias String name, @Filepath String path) {
        this.name = name;
        this.path = path;
    }

    public @Alias String getName() {
        return this.name;
    }

    public FsSymbolicDto setName(@Alias String value) {
        this.name = value;
        return this;
    }

    public @Filepath String getPath() {
        return this.path;
    }

    public FsSymbolicDto setPath(@Filepath String value) {
        this.path = value;
        return this;
    }
}
