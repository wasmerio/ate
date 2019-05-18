package com.tokera.ate.security.core;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.ArrayList;
import java.util.List;

import org.bouncycastle.pqc.crypto.ntru.NTRUParameters;
import org.bouncycastle.pqc.crypto.ntru.NTRUSigningKeyGenerationParameters;
import org.bouncycastle.pqc.crypto.ntru.NTRUSigningPrivateKeyParameters;
import org.bouncycastle.pqc.crypto.ntru.NTRUSigningPublicKeyParameters;
import org.bouncycastle.pqc.math.ntru.euclid.BigIntEuclidean;
import org.bouncycastle.pqc.math.ntru.polynomial.*;
import org.checkerframework.checker.nullness.qual.NonNull;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.checkerframework.framework.qual.DefaultQualifier;
import org.bouncycastle.crypto.AsymmetricCipherKeyPair;
import org.bouncycastle.crypto.KeyGenerationParameters;

import static java.math.BigInteger.ONE;
import static java.math.BigInteger.ZERO;

/**
 * Special version of the NTRU signing key generator that allows custom random number generators to be used instead
 * of the mandatory SecureRandom class.
 */
@DefaultQualifier(Nullable.class)
@SuppressWarnings({"argument.type.incompatible", "return.type.incompatible", "dereference.of.nullable", "iterating.over.nullable", "method.invocation.invalid", "override.return.invalid", "unnecessary.equals", "known.nonnull", "flowexpr.parse.error.postcondition", "unboxing.of.nullable", "accessing.nullable", "type.invalid.annotations.on.use", "switching.nullable", "initialization.fields.uninitialized"})
public class SigningKeyPairGenerator {

    private NTRUSigningKeyGenerationParameters params;

    public void init(KeyGenerationParameters param) {
        this.params = (NTRUSigningKeyGenerationParameters) param;
    }

    public @NonNull AsymmetricCipherKeyPair generateKeyPair(IRandom random) {
        List<NTRUSigningPrivateKeyParameters.Basis> b = new ArrayList<>();
        NTRUSigningPublicKeyParameters p = null;
        for (int k = params.B; k >= 0; k--) {
            NTRUSigningPrivateKeyParameters.Basis basis = generateBoundedBasis(random);
            b.add(basis);
            if (k == 0) {
                p = new NTRUSigningPublicKeyParameters(basis.h, params.getSigningParameters());
            }
        }
        return new AsymmetricCipherKeyPair(p, new NTRUSigningPrivateKeyParameters(b, p));
    }

    private void min(IntegerPolynomial f, IntegerPolynomial g, IntegerPolynomial F, IntegerPolynomial G, int N) {
        int E = 0;
        for (int j = 0; j < N; j++) {
            E += 2 * N * (f.coeffs[j] * f.coeffs[j] + g.coeffs[j] * g.coeffs[j]);
        }

        E -= 4;

        IntegerPolynomial u = (IntegerPolynomial) f.clone();
        IntegerPolynomial v = (IntegerPolynomial) g.clone();
        int j = 0;
        int k = 0;
        int maxAdjustment = N;
        while (k < maxAdjustment && j < N) {
            int D = 0;
            int i = 0;
            while (i < N) {
                int D1 = F.coeffs[i] * f.coeffs[i];
                int D2 = G.coeffs[i] * g.coeffs[i];
                int D3 = 4 * N * (D1 + D2);
                D += D3;
                i++;
            }

            int D1 = 4 * (F.sumCoeffs() + G.sumCoeffs());
            D -= D1;

            if (D > E) {
                F.sub(u);
                G.sub(v);
                k++;
                j = 0;
            } else if (D < -E) {
                F.add(u);
                G.add(v);
                k++;
                j = 0;
            }
            j++;
            u.rotate1();
            v.rotate1();
        }
    }

    private FGBasis generateBasis(IRandom random) {
        int N = params.N;
        int q = params.q;
        int d = params.d;
        int d1 = params.d1;
        int d2 = params.d2;
        int d3 = params.d3;
        int basisType = params.basisType;

        Polynomial f;
        IntegerPolynomial fInt;
        Polynomial g;
        IntegerPolynomial gInt;
        IntegerPolynomial fq;
        Resultant rf;
        Resultant rg;
        BigIntEuclidean r;

        int _2n1 = 2 * N + 1;
        boolean primeCheck = params.primeCheck;

        do {
            do {
                f = params.polyType == NTRUParameters.TERNARY_POLYNOMIAL_TYPE_SIMPLE ? PredictableSupportUtil.generateRandomDense(N, d + 1, d, random.getRandom()) : PredictableSupportUtil.generateRandomProduct(N, d1, d2, d3 + 1, d3, random.getRandom());
                fInt = f.toIntegerPolynomial();
            } while (primeCheck && fInt.resultant(_2n1).res.equals(ZERO));
            fq = fInt.invertFq(q);
        } while (fq == null);
        rf = fInt.resultant();

        do {
            do {
                do {
                    g = params.polyType == NTRUParameters.TERNARY_POLYNOMIAL_TYPE_SIMPLE ? PredictableSupportUtil.generateRandomDense(N, d + 1, d, random.getRandom()) : PredictableSupportUtil.generateRandomProduct(N, d1, d2, d3 + 1, d3, random.getRandom());
                    gInt = g.toIntegerPolynomial();
                } while (primeCheck && gInt.resultant(_2n1).res.equals(ZERO));
            } while (gInt.invertFq(q) == null);
            rg = gInt.resultant();
            r = BigIntEuclidean.calculate(rf.res, rg.res);
        } while (!r.gcd.equals(ONE));

        BigIntPolynomial A = (BigIntPolynomial) rf.rho.clone();
        A.mult(r.x.multiply(BigInteger.valueOf(q)));
        BigIntPolynomial B = (BigIntPolynomial) rg.rho.clone();
        B.mult(r.y.multiply(BigInteger.valueOf(-q)));

        BigIntPolynomial C;
        if (params.keyGenAlg == NTRUSigningKeyGenerationParameters.KEY_GEN_ALG_RESULTANT) {
            int[] fRevCoeffs = new int[N];
            int[] gRevCoeffs = new int[N];
            fRevCoeffs[0] = fInt.coeffs[0];
            gRevCoeffs[0] = gInt.coeffs[0];
            for (int i = 1; i < N; i++) {
                fRevCoeffs[i] = fInt.coeffs[N - i];
                gRevCoeffs[i] = gInt.coeffs[N - i];
            }
            IntegerPolynomial fRev = new IntegerPolynomial(fRevCoeffs);
            IntegerPolynomial gRev = new IntegerPolynomial(gRevCoeffs);

            IntegerPolynomial t = f.mult(fRev);
            t.add(g.mult(gRev));
            Resultant rt = t.resultant();
            C = fRev.mult(B);
            C.add(gRev.mult(A));
            C = C.mult(rt.rho);
            C.div(rt.res);
        } else {
            int log10N = 0;
            for (int i = 1; i < N; i *= 10) {
                log10N++;
            }

            BigDecimalPolynomial fInv = rf.rho.div(new BigDecimal(rf.res), B.getMaxCoeffLength() + 1 + log10N);
            BigDecimalPolynomial gInv = rg.rho.div(new BigDecimal(rg.res), A.getMaxCoeffLength() + 1 + log10N);

            BigDecimalPolynomial Cdec = fInv.mult(B);
            Cdec.add(gInv.mult(A));
            Cdec.halve();
            C = Cdec.round();
        }

        BigIntPolynomial F = (BigIntPolynomial) B.clone();
        F.sub(f.mult(C));
        BigIntPolynomial G = (BigIntPolynomial) A.clone();
        G.sub(g.mult(C));

        IntegerPolynomial FInt = new IntegerPolynomial(F);
        IntegerPolynomial GInt = new IntegerPolynomial(G);
        min(fInt, gInt, FInt, GInt, N);

        Polynomial fPrime;
        IntegerPolynomial h;
        if (basisType == NTRUSigningKeyGenerationParameters.BASIS_TYPE_STANDARD) {
            fPrime = FInt;
            h = g.mult(fq, q);
        } else {
            fPrime = g;
            h = FInt.mult(fq, q);
        }
        h.modPositive(q);

        return new FGBasis(f, fPrime, h, FInt, GInt, params);
    }

    public NTRUSigningPrivateKeyParameters.Basis generateBoundedBasis(IRandom random) {
        while (true) {
            FGBasis basis = generateBasis(random);
            if (basis.isOk()) {
                return basis;
            }
        }
    }

    public class FGBasis
            extends NTRUSigningPrivateKeyParameters.Basis {

        public IntegerPolynomial F;
        public IntegerPolynomial G;

        FGBasis(Polynomial f, Polynomial fPrime, IntegerPolynomial h, IntegerPolynomial F, IntegerPolynomial G, NTRUSigningKeyGenerationParameters params) {
            super(f, fPrime, h, params);
            this.F = F;
            this.G = G;
        }

        boolean isOk() {
            double keyNormBoundSq = params.keyNormBoundSq;
            int q = params.q;
            return (F.centeredNormSq(q) < keyNormBoundSq && G.centeredNormSq(q) < keyNormBoundSq);
        }
    }
}
