/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto.fs;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.DeviceName;

import javax.validation.constraints.NotNull;
import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;

/**
 * Represents a folder of files and sub directories
 *
 * @author root
 */
@YamlTag("dto.fs.mount")
public class FsMountDto {

    @JsonProperty
    @NotNull
    @Size(min=1, max=64)
    @Pattern(regexp = "^[a-zA-Z0-9_\\#\\-\\:\\@\\.]+$")
    private @Alias String name;
    @JsonProperty
    @NotNull
    @Size(min=1, max=15)
    @Pattern(regexp = "^[a-zA-Z][a-zA-Z0-9_]{1,15}$", message = "Invalid network name")
    private @DeviceName String dev;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public FsMountDto() {
    }

    public FsMountDto(String name, String dev) {
        this.name = name;
        this.dev = dev;
    }

    public @Alias String getName() {
        return this.name;
    }

    public FsMountDto setName(@Alias String value) {
        this.name = value;
        return this;
    }

    public @DeviceName String getDev() {
        return dev;
    }

    public FsMountDto setDev(@DeviceName String dev) {
        this.dev = dev;
        return this;
    }
}
