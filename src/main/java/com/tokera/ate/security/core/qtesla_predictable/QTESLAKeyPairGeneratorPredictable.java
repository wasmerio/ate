package com.tokera.ate.security.core.qtesla_predictable;

import com.tokera.ate.security.core.IRandom;
import org.bouncycastle.crypto.AsymmetricCipherKeyPair;
import org.bouncycastle.crypto.KeyGenerationParameters;
import org.bouncycastle.pqc.crypto.qtesla.QTESLAKeyGenerationParameters;
import org.bouncycastle.pqc.crypto.qtesla.QTESLAPrivateKeyParameters;
import org.bouncycastle.pqc.crypto.qtesla.QTESLAPublicKeyParameters;

/**
 * Key-pair generator for qTESLA keys.
 */
public final class QTESLAKeyPairGeneratorPredictable
{
    /**
     * qTESLA Security Category
     */
    private int securityCategory;
    private IRandom secureRandom;

    /**
     * Initialize the generator with a security category and a source of randomness.
     *
     * @param param a {@link QTESLAKeyGenerationParameters} object.
     */
    public void init(KeyGenerationParameters param, IRandom random)
    {
        QTESLAKeyGenerationParameters parameters = (QTESLAKeyGenerationParameters)param;

        this.secureRandom = random;
        this.securityCategory = parameters.getSecurityCategory();
    }

    /**
     * Generate a key-pair.
     *
     * @return a matching key-pair consisting of (QTESLAPublicKeyParameters, QTESLAPrivateKeyParameters).
     */
    public AsymmetricCipherKeyPair generateKeyPair()
    {
        byte[] privateKey = allocatePrivate(securityCategory);
        byte[] publicKey = allocatePublic(securityCategory);

        switch (securityCategory)
        {
            case QTESLASecurityCategory.HEURISTIC_I:
                QTESLA.generateKeyPairI(publicKey, privateKey, secureRandom);
                break;
            case QTESLASecurityCategory.HEURISTIC_III_SIZE:
                QTESLA.generateKeyPairIIISize(publicKey, privateKey, secureRandom);
                break;
            case QTESLASecurityCategory.HEURISTIC_III_SPEED:
                QTESLA.generateKeyPairIIISpeed(publicKey, privateKey, secureRandom);
                break;
            case QTESLASecurityCategory.PROVABLY_SECURE_I:
                QTESLA.generateKeyPairIP(publicKey, privateKey, secureRandom);
                break;
            case QTESLASecurityCategory.PROVABLY_SECURE_III:
                QTESLA.generateKeyPairIIIP(publicKey, privateKey, secureRandom);
                break;
            default:
                throw new IllegalArgumentException("unknown security category: " + securityCategory);
        }

        return new AsymmetricCipherKeyPair(new QTESLAPublicKeyParameters(securityCategory, publicKey), new QTESLAPrivateKeyParameters(securityCategory, privateKey));
    }

    private byte[] allocatePrivate(int securityCategory)
    {
        return new byte[QTESLASecurityCategory.getPrivateSize(securityCategory)];
    }

    private byte[] allocatePublic(int securityCategory)
    {
        return new byte[QTESLASecurityCategory.getPublicSize(securityCategory)];
    }
}