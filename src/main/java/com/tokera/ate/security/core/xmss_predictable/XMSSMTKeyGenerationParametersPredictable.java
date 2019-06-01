package com.tokera.ate.security.core.xmss_predictable;

import java.security.SecureRandom;

import org.bouncycastle.crypto.KeyGenerationParameters;

/**
 * XMSS^MT key-pair generation parameters.
 */
public final class XMSSMTKeyGenerationParametersPredictable
        extends KeyGenerationParameters
{
    private final XMSSMTParametersPredictable xmssmtParameters;

    /**
     * XMSSMT constructor...
     *
     * @param prng   Secure random to use.
     */
    public XMSSMTKeyGenerationParametersPredictable(XMSSMTParametersPredictable xmssmtParameters, SecureRandom prng)
    {
        super(prng,-1);

        this.xmssmtParameters = xmssmtParameters;
    }

    public XMSSMTParametersPredictable getParameters()
    {
        return xmssmtParameters;
    }
}

