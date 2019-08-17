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

import javax.enterprise.context.Dependent;

/**
 * Represents a encrypt key with a seed that was used to generate it which makes it easier to share
 */
@Dependent
@YamlTag("dto.encrypt.key.with.seed")
public class EncryptKeyWithSeedDto {

    @JsonProperty
    public MessagePrivateKeyDto key;
    @JsonProperty
    public String seed;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public EncryptKeyWithSeedDto() {
    }

    public EncryptKeyWithSeedDto(String seed, MessagePrivateKeyDto key) {
        this.key = key;
        this.seed = seed;
    }

    public EncryptKeyWithSeedDto(String seed) {
        this.seed = seed;
        this.key = AteDelegate.get().encryptor.genEncryptKeyFromSeed(seed);
    }

    public String publicHash() {
        return this.key.getPublicKeyHash();
    }
}
