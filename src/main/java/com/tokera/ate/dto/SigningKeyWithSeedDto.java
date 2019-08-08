/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.Claim;

import javax.enterprise.context.Dependent;
import javax.validation.constraints.NotNull;
import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;

/**
 * Represents a signing key with a seed that was used to generate it which makes it easier to share
 */
@Dependent
@YamlTag("dto.signing.key.with.seed")
public class SigningKeyWithSeedDto {

    @JsonProperty
    public MessagePrivateKeyDto key;
    @JsonProperty
    public String seed;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public SigningKeyWithSeedDto() {
    }

    public SigningKeyWithSeedDto(String seed, MessagePrivateKeyDto key) {
        this.key = key;
        this.seed = seed;
    }

    public SigningKeyWithSeedDto(String seed) {
        this.seed = seed;
        this.key = AteDelegate.get().encryptor.genSignKeyFromSeed(seed);
    }

    public String publicHash() {
        return this.key.getPublicKeyHash();
    }
}
