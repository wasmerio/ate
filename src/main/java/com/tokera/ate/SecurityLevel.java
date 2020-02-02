package com.tokera.ate;

import com.google.common.collect.Lists;
import com.tokera.ate.dao.enumerations.KeyType;

import java.util.List;

public class SecurityLevel {
    public final List<KeyType> signingTypes;
    public final List<KeyType> encryptTypes;
    public final int aesStrength;
    public final int signingStrength;
    public final int encryptionStrength;
    public final boolean automaticKeyRotation;
    public final boolean encryptToken;
    public final boolean signToken;
    public final int tokenExpiresMins;

    public SecurityLevel() {
        this.signingTypes = Lists.newArrayList(KeyType.qtesla, KeyType.rainbow);
        this.encryptTypes = Lists.newArrayList(KeyType.aes, KeyType.ntru);
        this.aesStrength = 256;
        this.signingStrength = 256;
        this.encryptionStrength = 256;
        this.automaticKeyRotation = true;
        this.encryptToken = true;
        this.signToken = true;
        this.tokenExpiresMins = 5;
    }

    public SecurityLevel(SecurityLevel other) {
        this.signingTypes = other.signingTypes;
        this.encryptTypes = other.encryptTypes;
        this.aesStrength = other.aesStrength;
        this.signingStrength = other.signingStrength;
        this.encryptionStrength = other.encryptionStrength;
        this.automaticKeyRotation = other.automaticKeyRotation;
        this.encryptToken = other.encryptToken;
        this.signToken = other.signToken;
        this.tokenExpiresMins = other.tokenExpiresMins;
    }

    public SecurityLevel(int aesStrength, int signingStrength, int encryptionStrength, boolean automaticKeyRotation, List<KeyType> signingTypes, List<KeyType> encryptTypes, boolean encryptToken, boolean signToken, int tokenExpiresMins) {
        this.automaticKeyRotation = automaticKeyRotation;
        this.signingTypes = signingTypes;
        this.encryptTypes = encryptTypes;
        this.aesStrength = aesStrength;
        this.signingStrength = signingStrength;
        this.encryptionStrength = encryptionStrength;
        this.encryptToken = encryptToken;
        this.signToken = signToken;
        this.tokenExpiresMins = tokenExpiresMins;
    }

    public static SecurityLevel RidiculouslySecure = new SecurityLevel(256, 512, 512, true, Lists.newArrayList(KeyType.qtesla, KeyType.rainbow), Lists.newArrayList(KeyType.aes, KeyType.ntru), true, true, 1);
    public static SecurityLevel VeryHighlySecure = new SecurityLevel(256, 256, 256, true, Lists.newArrayList(KeyType.qtesla), Lists.newArrayList(KeyType.aes), true, true, 5);
    public static SecurityLevel HighlySecure = new SecurityLevel(192, 192, 256, false, Lists.newArrayList(KeyType.qtesla), Lists.newArrayList(KeyType.aes), true, true, 20);
    public static SecurityLevel ModeratelySecure = new SecurityLevel(128, 128, 192, false, Lists.newArrayList(KeyType.qtesla), Lists.newArrayList(KeyType.aes), false, true, 0);
    public static SecurityLevel PoorlySecure = new SecurityLevel(128, 64, 128, false, Lists.newArrayList(KeyType.qtesla), Lists.newArrayList(KeyType.aes), false, false, 0);

    public SecurityLevel withAesStrength(int aesStrength) {
        return new SecurityLevel(aesStrength,
                signingStrength,
                encryptionStrength,
                automaticKeyRotation,
                signingTypes,
                encryptTypes,
                encryptToken,
                signToken,
                tokenExpiresMins);
    }

    public SecurityLevel withSigningStrength(int signingStrength) {
        return new SecurityLevel(aesStrength,
                signingStrength,
                encryptionStrength,
                automaticKeyRotation,
                signingTypes,
                encryptTypes,
                encryptToken,
                signToken,
                tokenExpiresMins);
    }

    public SecurityLevel withEncryptionStrength(int encryptionStrength) {
        return new SecurityLevel(aesStrength,
                signingStrength,
                encryptionStrength,
                automaticKeyRotation,
                signingTypes,
                encryptTypes,
                encryptToken,
                signToken,
                tokenExpiresMins);
    }

    public SecurityLevel withAutomaticKeyRotation(boolean automaticKeyRotation) {
        return new SecurityLevel(aesStrength,
                signingStrength,
                encryptionStrength,
                automaticKeyRotation,
                signingTypes,
                encryptTypes,
                encryptToken,
                signToken,
                tokenExpiresMins);
    }

    public SecurityLevel withSigningTypes(List<KeyType> signingTypes) {
        return new SecurityLevel(aesStrength,
                signingStrength,
                encryptionStrength,
                automaticKeyRotation,
                signingTypes,
                encryptTypes,
                encryptToken,
                signToken,
                tokenExpiresMins);
    }

    public SecurityLevel withEncryptTypes(List<KeyType> encryptTypes) {
        return new SecurityLevel(aesStrength,
                signingStrength,
                encryptionStrength,
                automaticKeyRotation,
                signingTypes,
                encryptTypes,
                encryptToken,
                signToken,
                tokenExpiresMins);
    }

    public SecurityLevel withEncryptToken(boolean encryptToken) {
        return new SecurityLevel(aesStrength,
                signingStrength,
                encryptionStrength,
                automaticKeyRotation,
                signingTypes,
                encryptTypes,
                encryptToken,
                signToken,
                tokenExpiresMins);
    }

    public SecurityLevel withSignToken(boolean signToken) {
        return new SecurityLevel(aesStrength,
                signingStrength,
                encryptionStrength,
                automaticKeyRotation,
                signingTypes,
                encryptTypes,
                encryptToken,
                signToken,
                tokenExpiresMins);
    }

    public SecurityLevel withTokenExpiresMins(int tokenExpiresMins) {
        return new SecurityLevel(aesStrength,
                signingStrength,
                encryptionStrength,
                automaticKeyRotation,
                signingTypes,
                encryptTypes,
                encryptToken,
                signToken,
                tokenExpiresMins);
    }
}
