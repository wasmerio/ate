package com.tokera.ate.security.core.ntru_predictable;

import com.tokera.ate.security.core.IRandom;
import org.bouncycastle.pqc.crypto.ntru.NTRUEncryptionKeyGenerationParameters;
import org.bouncycastle.pqc.crypto.ntru.NTRUEncryptionPrivateKeyParameters;
import org.bouncycastle.pqc.crypto.ntru.NTRUEncryptionPublicKeyParameters;
import org.bouncycastle.pqc.crypto.ntru.NTRUParameters;
import org.bouncycastle.pqc.math.ntru.polynomial.DenseTernaryPolynomial;
import org.bouncycastle.pqc.math.ntru.polynomial.IntegerPolynomial;
import org.bouncycastle.pqc.math.ntru.polynomial.Polynomial;
import org.checkerframework.checker.nullness.qual.NonNull;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.checkerframework.framework.qual.DefaultQualifier;
import org.bouncycastle.crypto.AsymmetricCipherKeyPair;
import org.bouncycastle.crypto.KeyGenerationParameters;

/**
 * Special version of the NTRU encryption key generator that allows custom random number generators to be used instead
 * of the mandatory SecureRandom class.
 */
@DefaultQualifier(Nullable.class)
@SuppressWarnings({"argument.type.incompatible", "return.type.incompatible", "dereference.of.nullable", "iterating.over.nullable", "method.invocation.invalid", "override.return.invalid", "unnecessary.equals", "known.nonnull", "flowexpr.parse.error.postcondition", "unboxing.of.nullable", "accessing.nullable", "type.invalid.annotations.on.use", "switching.nullable", "initialization.fields.uninitialized"})
public class EncryptionKeyPairGenerator {

    private NTRUEncryptionKeyGenerationParameters params;

    public void init(KeyGenerationParameters param) {
        this.params = (NTRUEncryptionKeyGenerationParameters) param;
    }

    public @NonNull AsymmetricCipherKeyPair generateKeyPair(IRandom random) {
        int N = params.N;
        int q = params.q;
        int df = params.df;
        int df1 = params.df1;
        int df2 = params.df2;
        int df3 = params.df3;
        int dg = params.dg;
        boolean fastFp = params.fastFp;
        boolean sparse = params.sparse;

        Polynomial t;
        IntegerPolynomial fq;
        IntegerPolynomial fp = null;

        while (true) {
            IntegerPolynomial f;

            if (fastFp) {
                t = params.polyType == NTRUParameters.TERNARY_POLYNOMIAL_TYPE_SIMPLE ? SupportUtil.generateRandomTernary(N, df, df, sparse, random.getRandom()) : SupportUtil.generateRandomProduct(N, df1, df2, df3, df3, random.getRandom());
                f = t.toIntegerPolynomial();
                f.mult(3);
                f.coeffs[0] += 1;
            } else {
                t = params.polyType == NTRUParameters.TERNARY_POLYNOMIAL_TYPE_SIMPLE ? SupportUtil.generateRandomTernary(N, df, df - 1, sparse, random.getRandom()) : SupportUtil.generateRandomProduct(N, df1, df2, df3, df3 - 1, random.getRandom());
                f = t.toIntegerPolynomial();
                fp = f.invertF3();
                if (fp == null) {
                    continue;
                }
            }

            fq = f.invertFq(q);
            if (fq == null) {
                continue;
            }
            break;
        }

        if (fastFp) {
            fp = new IntegerPolynomial(N);
            fp.coeffs[0] = 1;
        }

        DenseTernaryPolynomial g;
        while (true) {
            g = SupportUtil.generateRandomDense(N, dg, dg - 1, random.getRandom());
            if (g.invertFq(q) != null) {
                break;
            }
        }

        IntegerPolynomial h = g.mult(fq, q);
        h.mult3(q);
        h.ensurePositive(q);
        g.clear();
        fq.clear();

        NTRUEncryptionPrivateKeyParameters priv = new NTRUEncryptionPrivateKeyParameters(h, t, fp, params.getEncryptionParameters());
        NTRUEncryptionPublicKeyParameters pub = new NTRUEncryptionPublicKeyParameters(h, params.getEncryptionParameters());
        return new AsymmetricCipherKeyPair(pub, priv);
    }
}
