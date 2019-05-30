package com.tokera.ate.security.core.newhope_predictable;

import com.tokera.ate.security.core.IRandom;
import org.bouncycastle.crypto.AsymmetricCipherKeyPair;
import org.bouncycastle.pqc.crypto.newhope.NHPrivateKeyParameters;
import org.bouncycastle.pqc.crypto.newhope.NHPublicKeyParameters;

public class NHKeyPairGeneratorPredictable {
    private IRandom random;

    public NHKeyPairGeneratorPredictable() {
    }

    public void init(IRandom random) {
        this.random = random;
    }

    public AsymmetricCipherKeyPair generateKeyPair() {
        byte[] pubData = new byte[NewHope.SENDA_BYTES];
        short[] secData = new short[NewHope.POLY_SIZE];

        NewHope.keygen(random, pubData, secData);

        return new AsymmetricCipherKeyPair(new NHPublicKeyParameters(pubData), new NHPrivateKeyParameters(secData));
    }
}
