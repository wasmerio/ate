package com.tokera.ate.security.core;

import org.bouncycastle.crypto.AsymmetricCipherKeyPair;
import org.bouncycastle.pqc.crypto.newhope.NHPrivateKeyParameters;
import org.bouncycastle.pqc.crypto.newhope.NHPublicKeyParameters;

public class NHKeyPairGeneratorPredictable {
    private PredictablyRandom random;

    public NHKeyPairGeneratorPredictable() {
    }

    public void init(PredictablyRandom random) {
        this.random = random;
    }

    public AsymmetricCipherKeyPair generateKeyPair() {
        byte[] pubData = new byte[NewHopePredictable.SENDA_BYTES];
        short[] secData = new short[NewHopePredictable.POLY_SIZE];

        NewHopePredictable.keygen(random, pubData, secData);

        return new AsymmetricCipherKeyPair(new NHPublicKeyParameters(pubData), new NHPrivateKeyParameters(secData));
    }
}
