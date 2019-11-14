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
 * Represents a binary
 */
@YamlTag("dto.fs.binary")
public class FsBinaryDto {

    @JsonProperty
    @NotNull
    @Size(min=1, max=64)
    @Pattern(regexp = "^[a-zA-Z0-9_\\#\\-\\:\\@\\.]+$")
    private @Alias String name;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public FsBinaryDto() {
    }

    public FsBinaryDto(String name) {
        this.name = name;
    }

    public @Alias String getName() {
        return this.name;
    }

    public FsBinaryDto setName(@Alias String value) {
        this.name = value;
        return this;
    }
}