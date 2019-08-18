/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.google.common.collect.Lists;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.dao.enumerations.KeyType;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import org.checkerframework.checker.nullness.qual.Nullable;

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

    public static int KEYSIZE = 192;
    public static Iterable<KeyType> KEYTYPE = Lists.newArrayList(KeyType.ntru);

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public EncryptKeyWithSeedDto() {
    }

    public EncryptKeyWithSeedDto(MessagePrivateKeyDto key, String seed) {
        this.key = key;
        this.seed = seed;
    }

    public EncryptKeyWithSeedDto(String seed) {
        this(seed, null);
    }

    public EncryptKeyWithSeedDto(String seed, @Nullable String alias) {
        this.seed = seed;
        this.key = AteDelegate.get().encryptor.genEncryptKeyFromSeed(KEYSIZE, KEYTYPE, seed);
        if (alias != null) {
            this.key.setAlias(alias);
        }
    }

    public String publicHash() {
        return this.key.getPublicKeyHash();
    }

    public @Nullable String getAlias() {
        return this.key.getAlias();
    }

    public void setAlias(String alias) {
        this.key.setAlias(alias);
    }
}
