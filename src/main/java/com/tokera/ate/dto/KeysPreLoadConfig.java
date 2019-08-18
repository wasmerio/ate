package com.tokera.ate.dto;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.units.Secret;

import javax.enterprise.context.Dependent;
import java.util.ArrayList;

@Dependent
@YamlTag("preload.keys.config")
public class KeysPreLoadConfig {

    @JsonProperty
    public final ArrayList<MessagePrivateKeyDto> sign64 = new ArrayList<>();
    @JsonProperty
    public final ArrayList<MessagePrivateKeyDto> sign128 = new ArrayList<>();
    @JsonProperty
    public final ArrayList<MessagePrivateKeyDto> sign256 = new ArrayList<>();
    @JsonProperty
    public final ArrayList<PrivateKeyWithSeedDto> signAndSeed64 = new ArrayList<>();
    @JsonProperty
    public final ArrayList<PrivateKeyWithSeedDto> signAndSeed128 = new ArrayList<>();
    @JsonProperty
    public final ArrayList<PrivateKeyWithSeedDto> signAndSeed256 = new ArrayList<>();
    @JsonProperty
    public final ArrayList<PrivateKeyWithSeedDto> encryptAndSeed128 = new ArrayList<>();
    @JsonProperty
    public final ArrayList<PrivateKeyWithSeedDto> encryptAndSeed256 = new ArrayList<>();
    @JsonProperty
    public final ArrayList<MessagePrivateKeyDto> encrypt128 = new ArrayList<>();
    @JsonProperty
    public final ArrayList<MessagePrivateKeyDto> encrypt256 = new ArrayList<>();
    @JsonProperty
    public final ArrayList<@Secret String> aes128 = new ArrayList<>();
    @JsonProperty
    public final ArrayList<@Secret String> aes256 = new ArrayList<>();
    @JsonProperty
    public final ArrayList<@Secret String> aes512 = new ArrayList<>();
    @JsonProperty
    public final ArrayList<@Secret String> salt = new ArrayList<>();
}
