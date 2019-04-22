package com.tokera.ate.security.core;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Random;

import org.checkerframework.checker.nullness.qual.Nullable;
import org.checkerframework.framework.qual.DefaultQualifier;
import org.spongycastle.math.ntru.polynomial.DenseTernaryPolynomial;
import org.spongycastle.math.ntru.polynomial.ProductFormPolynomial;
import org.spongycastle.math.ntru.polynomial.SparseTernaryPolynomial;
import org.spongycastle.math.ntru.polynomial.TernaryPolynomial;

@DefaultQualifier(Nullable.class)
@SuppressWarnings({"argument.type.incompatible", "return.type.incompatible", "dereference.of.nullable", "iterating.over.nullable", "method.invocation.invalid", "override.return.invalid", "unnecessary.equals", "known.nonnull", "flowexpr.parse.error.postcondition", "unboxing.of.nullable", "accessing.nullable", "type.invalid.annotations.on.use", "switching.nullable", "initialization.fields.uninitialized"})
public class PredictableSupportUtil {

    public static TernaryPolynomial generateRandomTernary(int N, int numOnes, int numNegOnes, boolean sparse, Random random) {
        if (sparse) {
            return generateRandomSparse(N, numOnes, numNegOnes, random);
        } else {
            return generateRandomDense(N, numOnes, numNegOnes, random);
        }
    }

    public static int[] generateRandomTernary(int N, int numOnes, int numNegOnes, Random random) {
        Integer one = 1;
        Integer minusOne = -1;
        Integer zero = 0;

        List list = new ArrayList();
        for (int i = 0; i < numOnes; i++) {
            list.add(one);
        }
        for (int i = 0; i < numNegOnes; i++) {
            list.add(minusOne);
        }
        while (list.size() < N) {
            list.add(zero);
        }

        Collections.shuffle(list, random);

        int[] arr = new int[N];
        for (int i = 0; i < N; i++) {
            arr[i] = ((Integer) list.get(i));
        }
        return arr;
    }

    public static SparseTernaryPolynomial generateRandomSparse(int N, int numOnes, int numNegOnes, Random random) {
        int[] coeffs = generateRandomTernary(N, numOnes, numNegOnes, random);
        return new SparseTernaryPolynomial(coeffs);
    }

    public static DenseTernaryPolynomial generateRandomDense(int N, int numOnes, int numNegOnes, Random random) {
        int[] coeffs = generateRandomTernary(N, numOnes, numNegOnes, random);
        return new DenseTernaryPolynomial(coeffs);
    }

    public static ProductFormPolynomial generateRandomProduct(int N, int df1, int df2, int df3Ones, int df3NegOnes, Random random) {
        SparseTernaryPolynomial f1 = generateRandomSparse(N, df1, df1, random);
        SparseTernaryPolynomial f2 = generateRandomSparse(N, df2, df2, random);
        SparseTernaryPolynomial f3 = generateRandomSparse(N, df3Ones, df3NegOnes, random);
        return new ProductFormPolynomial(f1, f2, f3);
    }
}
