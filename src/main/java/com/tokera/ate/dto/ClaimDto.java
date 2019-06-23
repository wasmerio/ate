/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.Claim;

import javax.validation.constraints.NotNull;
import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;

/**
 * Represents a claim within a token, claims have both a key and value.
 */
@YamlTag("dto.claim")
public class ClaimDto {

    @JsonProperty
    @NotNull
    @Size(min=1, max=64)
    @Pattern(regexp = "^[a-zA-Z0-9_\\#\\-\\:\\@\\.]+$")
    private @Alias String key;
    @JsonProperty
    @NotNull
    @Size(min=1)
    private @Claim String value;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public ClaimDto() {
    }

    public ClaimDto(@Alias String key, @Claim String val) {
        this.key = key;
        this.value = val;
    }

    public @Alias String getKey() {
        return this.key;
    }

    public void setKey(@Alias String value) {
        this.key = value;
    }

    public @Claim String getValue() {
        return this.value;
    }

    public void setValue(@Claim String value) {
        this.value = value;
    }
}
