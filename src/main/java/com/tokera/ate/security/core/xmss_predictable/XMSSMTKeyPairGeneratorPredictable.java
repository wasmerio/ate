package com.tokera.ate.security.core.xmss_predictable;

import com.tokera.ate.security.core.IRandom;
import org.bouncycastle.crypto.AsymmetricCipherKeyPair;

/**
 * Key pair generator for XMSS^MT keys.
 */
public final class XMSSMTKeyPairGeneratorPredictable
{
    private XMSSMTParameters params;
    private XMSSParameters xmssParams;

    private IRandom prng;


    /**
     * Base constructor...
     */
    public XMSSMTKeyPairGeneratorPredictable()
    {
    }

    public void init(
            XMSSMTKeyGenerationParameters parameters, IRandom random)
    {
        prng = random;
        this.params = parameters.getParameters();
        this.xmssParams = params.getXMSSParameters();
    }

    /**
     * Generate a new XMSSMT private key / public key pair.
     */
    public AsymmetricCipherKeyPair generateKeyPair()
    {
        XMSSMTPrivateKeyParametersPredictable privateKey;
        XMSSMTPublicKeyParametersPredictable publicKey;

        /* generate XMSSMT private key */
        privateKey = generatePrivateKey(new XMSSMTPrivateKeyParametersPredictable.Builder(params).build().getBDSState());

        /* import to xmss */
        xmssParams.getWOTSPlus().importKeys(new byte[params.getDigestSize()], privateKey.getPublicSeed());

        /* get root */
        int rootLayerIndex = params.getLayers() - 1;
        OTSHashAddress otsHashAddress = (OTSHashAddress)new OTSHashAddress.Builder().withLayerAddress(rootLayerIndex)
                .build();

        /* store BDS instance of root xmss instance */
        BDS bdsRoot = new BDS(xmssParams, privateKey.getPublicSeed(), privateKey.getSecretKeySeed(), otsHashAddress);
        XMSSNode root = bdsRoot.getRoot();
        privateKey.getBDSState().put(rootLayerIndex, bdsRoot);

        /* set XMSS^MT root / create public key */
        privateKey = new XMSSMTPrivateKeyParametersPredictable.Builder(params).withSecretKeySeed(privateKey.getSecretKeySeed())
                .withSecretKeyPRF(privateKey.getSecretKeyPRF()).withPublicSeed(privateKey.getPublicSeed())
                .withRoot(root.getValue()).withBDSState(privateKey.getBDSState()).build();
        publicKey = new XMSSMTPublicKeyParametersPredictable.Builder(params).withRoot(root.getValue())
                .withPublicSeed(privateKey.getPublicSeed()).build();

        return new AsymmetricCipherKeyPair(publicKey, privateKey);
    }

    private XMSSMTPrivateKeyParametersPredictable generatePrivateKey(BDSStateMap bdsState)
    {
        int n = params.getDigestSize();
        byte[] secretKeySeed = new byte[n];
        prng.getRandom().nextBytes(secretKeySeed);
        byte[] secretKeyPRF = new byte[n];
        prng.getRandom().nextBytes(secretKeyPRF);
        byte[] publicSeed = new byte[n];
        prng.getRandom().nextBytes(publicSeed);

        XMSSMTPrivateKeyParametersPredictable privateKey = null;

        privateKey = new XMSSMTPrivateKeyParametersPredictable.Builder(params).withSecretKeySeed(secretKeySeed)
                .withSecretKeyPRF(secretKeyPRF).withPublicSeed(publicSeed)
                .withBDSState(bdsState).build();

        return privateKey;
    }
}

