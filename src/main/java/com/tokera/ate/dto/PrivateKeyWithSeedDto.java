/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.databind.annotation.JsonDeserialize;
import com.fasterxml.jackson.databind.annotation.JsonSerialize;
import com.google.common.collect.Lists;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.dao.enumerations.KeyType;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.enumerations.PrivateKeyType;
import com.tokera.ate.providers.PrivateKeyWithSeedJsonDeserializer;
import com.tokera.ate.providers.PrivateKeyWithSeedJsonSerializer;
import com.tokera.ate.providers.TokenJsonDeserializer;
import com.tokera.ate.providers.TokenJsonSerializer;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.Response;
import java.util.ArrayList;
import java.util.List;
import java.util.stream.Collectors;

/**
 * Represents a encrypt key with a seed that was used to generate it which makes it easier to share
 */
@Dependent
@YamlTag("dto.key.with.seed")
@JsonSerialize(using = PrivateKeyWithSeedJsonSerializer.class)
@JsonDeserialize(using = PrivateKeyWithSeedJsonDeserializer.class)
public class PrivateKeyWithSeedDto {

    @JsonProperty
    private @Nullable String alias;
    @JsonProperty
    public final String seed;
    @JsonProperty
    public final PrivateKeyType type;
    @JsonProperty
    public final int keySize;
    @JsonProperty
    public final List<KeyType> algs;

    @JsonIgnore
    private transient MessagePrivateKeyDto key;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public PrivateKeyWithSeedDto() {
        AteDelegate d = AteDelegate.get();
        this.seed = d.encryptor.generateSecret64(d.bootstrapConfig.getDefaultEncryptionStrength());
        this.key = null;
        this.alias = null;
        this.type = PrivateKeyType.read;
        this.keySize = d.bootstrapConfig.getDefaultEncryptionStrength();
        this.algs = d.bootstrapConfig.getDefaultEncryptTypes();
    }

    public PrivateKeyWithSeedDto(PrivateKeyWithSeedDto a) {
        this.seed = a.seed;
        this.key = null;
        this.alias = a.alias;
        this.type = a.type;
        this.keySize = a.keySize;
        this.algs = a.algs;
    }

    public PrivateKeyWithSeedDto(PrivateKeyWithSeedDto a, String newAlias) {
        this(a);
        this.alias = newAlias;
    }

    public PrivateKeyWithSeedDto(PrivateKeyType keyType) {
        this(keyType, AteDelegate.get().encryptor.generateSecret64(), null);
    }

    public PrivateKeyWithSeedDto(PrivateKeyType keyType, String seed) {
        this(keyType, seed, null);
    }

    public PrivateKeyWithSeedDto(PrivateKeyType keyType, String seed, @Nullable String alias) {
        this.seed = seed;
        this.key = null;
        if (alias != null && alias.length() > 0) {
            this.alias = alias;
        } else {
            this.alias = null;
        }
        this.type = keyType;

        AteDelegate d = AteDelegate.get();
        switch (keyType) {
            default:
            case read: {
                this.keySize = d.bootstrapConfig.getDefaultEncryptionStrength();
                this.algs = d.bootstrapConfig.getDefaultEncryptTypes();
                break;
            }
            case write: {
                this.keySize = d.bootstrapConfig.getDefaultSigningStrength();
                this.algs = d.bootstrapConfig.getDefaultSigningTypes();
                break;
            }
        }
    }

    public PrivateKeyWithSeedDto(PrivateKeyType keyType, String seed, int keySize, List<KeyType> algs, @Nullable String alias) {
        this.seed = seed;
        this.key = null;
        if (alias != null && alias.length() > 0) {
            this.alias = alias;
        } else {
            this.alias = null;
        }
        this.type = keyType;
        this.keySize = keySize;
        this.algs = algs;
    }

    @JsonIgnore
    public MessagePrivateKeyDto key() {
        if (this.key != null) return this.key;

        AteDelegate d = AteDelegate.get();
        MessagePrivateKeyDto ret;
        switch (this.type) {
            case read: {
                ret = d.encryptor.genEncryptKeyFromSeed(this.keySize, this.algs, this.seed);
                break;
            }
            case write: {
                ret = d.encryptor.genSignKeyFromSeed(this.keySize, this.algs, this.seed);
                break;
            }
            default: {
                throw new WebApplicationException("Unknown private key type: " + this.type, Response.Status.INTERNAL_SERVER_ERROR);
            }
        }

        if (this.alias != null) {
            ret.setAlias(this.alias);
        }

        this.key = ret;
        return ret;
    }

    @JsonIgnore
    public String publicHash() {
        return key().getPublicKeyHash();
    }

    @JsonIgnore
    public @Nullable String alias() {
        return key().getAlias();
    }

    @JsonIgnore
    public void setAlias(String alias) {
        this.alias = alias;
        if (this.key != null) {
            key().setAlias(alias);
        }
    }

    @JsonIgnore
    public String serialize() {
        StringBuilder sb = new StringBuilder();
        sb.append(this.type.shortName());
        sb.append(":");
        sb.append(this.keySize);
        sb.append(":");
        boolean firstAlg = true;
        for (KeyType alg : this.algs) {
            if (firstAlg == true) {
                firstAlg = false;
            } else {
                sb.append(",");
            }
            sb.append(alg.name());
        }
        sb.append(":");
        sb.append(this.seed);
        if (this.alias != null) {
            sb.append(":");
            sb.append(this.alias);
        }

        return sb.toString();
    }

    public static @Nullable PrivateKeyWithSeedDto deserialize(String val) {
        if (val == null) return null;
        if (val.length() <= 0) return null;

        String[] comps = val.split(":");
        if (comps.length >= 4) {
            PrivateKeyType type = PrivateKeyType.parse(comps[0]);
            Integer size = Integer.parseInt(comps[1]);
            List<KeyType> algs = new ArrayList<>();
            for (String alg : comps[2].split(",")) {
                algs.add(KeyType.valueOf(alg));
            }
            String seed = comps[3];
            String alias = null;
            if (comps.length >= 5) {
                alias = comps[4];
            }

            return new PrivateKeyWithSeedDto(type, seed, size, algs, alias);
        }
        throw new WebApplicationException("Failed to parse the string [" + val + "] into a PrivateKeyWithSeedDto.", Response.Status.INTERNAL_SERVER_ERROR);
    }

    @Override
    public String toString() {
        return serialize();
    }

    @Override
    public int hashCode() {
        return serialize().hashCode();
    }

    @Override
    public boolean equals(Object other) {
        if (other == null) return false;
        if (other instanceof PrivateKeyWithSeedDto) {
            if (serialize().equals(((PrivateKeyWithSeedDto) other).serialize()) == false) return false;
            return true;
        }
        return false;
    }
}
