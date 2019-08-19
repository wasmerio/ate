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
import java.io.UnsupportedEncodingException;
import java.net.URLDecoder;
import java.net.URLEncoder;
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
    private String seed;
    @JsonProperty
    private PrivateKeyType type;
    @JsonProperty
    private int keySize;
    @JsonProperty
    private List<KeyType> algs;
    @JsonProperty
    private @Nullable String publicHash;

    @JsonIgnore
    private transient MessagePrivateKeyDto key;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public PrivateKeyWithSeedDto() {
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

    public PrivateKeyWithSeedDto(PrivateKeyType keyType, int keySize) {
        this(keyType, keySize, null);
    }

    public PrivateKeyWithSeedDto(PrivateKeyType keyType, int keySize, @Nullable String alias) {
        this(keyType, AteDelegate.get().encryptor.generateSecret64(), keySize,
             (keyType == PrivateKeyType.write ? AteDelegate.get().bootstrapConfig.getDefaultSigningTypes() : AteDelegate.get().bootstrapConfig.getDefaultEncryptTypes()),
             null, alias);
    }

    public PrivateKeyWithSeedDto(PrivateKeyType keyType, String seed) {
        this(keyType, seed, null);
        assert seed == null || seed.contains(":") == false;
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

        assert seed == null || seed.contains(":") == false;
    }

    public PrivateKeyWithSeedDto(PrivateKeyType keyType, String seed, int keySize, KeyType alg, @Nullable String publicHash, @Nullable String alias) {
        this.seed = seed;
        this.key = null;
        if (alias != null && alias.length() > 0) {
            this.alias = alias;
        } else {
            this.alias = null;
        }
        this.type = keyType;
        this.keySize = keySize;
        this.algs = new ArrayList<>();
        this.algs.add(alg);
        this.publicHash = publicHash;

        assert seed == null || seed.contains(":") == false;
    }

    public PrivateKeyWithSeedDto(PrivateKeyType keyType, String seed, int keySize, List<KeyType> algs, @Nullable String publicHash, @Nullable String alias) {
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
        this.publicHash = publicHash;

        assert seed == null || seed.contains(":") == false;
    }

    @JsonIgnore
    public MessagePrivateKeyDto key() {
        if (this.key != null) return this.key;

        synchronized (this) {
            if (this.key != null) return this.key;

            AteDelegate d = AteDelegate.get();
            MessagePrivateKeyDto ret = d.encryptor.genKeyFromSeed(this);
            this.key = ret;
            return ret;
        }
    }

    @JsonIgnore
    public String publicHash() {
        if (publicHash != null) return publicHash;
        String ret = key().getPublicKeyHash();
        publicHash = ret;
        return ret;
    }

    @JsonIgnore
    public @Nullable String alias() {
        return this.alias;
    }

    @JsonIgnore
    public String aliasOrHash() {
        if (this.alias != null) {
            return this.alias;
        }
        return publicHash();
    }

    @JsonIgnore
    public void setAlias(String alias) {
        this.alias = alias;
        if (this.key != null) {
            key().setAlias(alias);
        }
    }

    @JsonIgnore
    public String seed() {
        return this.seed;
    }

    @JsonIgnore
    public PrivateKeyType type() {
        return this.type;
    }

    @JsonIgnore
    public int keySize() {
        return this.keySize;
    }

    @JsonIgnore
    public Iterable<KeyType> algs() {
        return this.algs;
    }

    @JsonIgnore
    public String serialize() {
        return serialize(true);
    }

    @JsonIgnore
    public String serialize(boolean includePublicHash) {
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
        sb.append(":");
        if (includePublicHash && this.publicHash != null) {
            sb.append(this.publicHash);
        }
        sb.append(":");
        if (this.alias != null) {
            try {
                sb.append(URLEncoder.encode(this.alias, "UTF-8"));
            } catch (UnsupportedEncodingException e) {
                throw new WebApplicationException(e);
            }
        }

        return sb.toString();
    }

    public static @Nullable PrivateKeyWithSeedDto deserialize(String val) {
        if (val == null) return null;
        if (val.length() <= 0) return null;

        String[] comps = val.split(":", -1);
        if (comps.length >= 6) {
            PrivateKeyType type = PrivateKeyType.parse(comps[0]);
            Integer size = Integer.parseInt(comps[1]);
            List<KeyType> algs = new ArrayList<>();
            for (String alg : comps[2].split(",")) {
                algs.add(KeyType.valueOf(alg));
            }
            String seed = comps[3];
            String alias = null;
            String publicKey = null;
            if (comps[4].length() > 0) {
                publicKey = comps[4];
            }
            if (comps[5].length() > 0) {
                try {
                    alias = URLDecoder.decode(comps[5], "UTF-8");
                } catch (UnsupportedEncodingException e) {
                    throw new WebApplicationException(e);
                }
            }

            return new PrivateKeyWithSeedDto(type, seed, size, algs, publicKey, alias);
        }
        throw new WebApplicationException("Failed to parse the string [" + val + "] into a PrivateKeyWithSeedDto.", Response.Status.INTERNAL_SERVER_ERROR);
    }

    @Override
    public String toString() {
        return serialize();
    }

    @Override
    public int hashCode() {
        return serialize(false).hashCode();
    }

    @Override
    public boolean equals(Object other) {
        if (other == null) return false;
        if (other instanceof PrivateKeyWithSeedDto) {
            if (serialize(false).equals(((PrivateKeyWithSeedDto) other).serialize(false)) == false) return false;
            return true;
        }
        return false;
    }
}
