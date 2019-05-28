package com.tokera.ate.security.core;

import org.bouncycastle.crypto.digests.SHA3Digest;


/**
 * This implementation is based heavily on the C reference implementation from https://cryptojedi.org/crypto/index.shtml.
 */
class NewHopePredictable
{
    private static final boolean STATISTICAL_TEST = false;

    public static final int AGREEMENT_SIZE = 32;
    public static final int POLY_SIZE = NHParamsPredictable.N;
    public static final int SENDA_BYTES = NHParamsPredictable.POLY_BYTES + NHParamsPredictable.SEED_BYTES;
    public static final int SENDB_BYTES = NHParamsPredictable.POLY_BYTES + NHParamsPredictable.REC_BYTES;

    public static void keygen(PredictablyRandom rand, byte[] send, short[] sk)
    {
        byte[] seed = new byte[NHParamsPredictable.SEED_BYTES];
        rand.getRandom().nextBytes(seed);

        sha3(seed);     // don't expose RNG output

        short[] a = new short[NHParamsPredictable.N];
        generateA(a, seed);

        byte[] noiseSeed = new byte[32];
        rand.getRandom().nextBytes(noiseSeed);

        NHPolyPredictable.getNoise(sk, noiseSeed, (byte)0);
        NHPolyPredictable.toNTT(sk);

        short[] e = new short[NHParamsPredictable.N];
        NHPolyPredictable.getNoise(e, noiseSeed, (byte)1);
        NHPolyPredictable.toNTT(e);

        short[] r = new short[NHParamsPredictable.N];
        NHPolyPredictable.pointWise(a, sk, r);

        short[] pk = new short[NHParamsPredictable.N];
        NHPolyPredictable.add(r, e, pk);

        encodeA(send, pk, seed);
    }

    public static void sharedB(PredictablyRandom rand, byte[] sharedKey, byte[] send, byte[] received)
    {
        short[] pkA = new short[NHParamsPredictable.N];
        byte[] seed = new byte[NHParamsPredictable.SEED_BYTES];
        decodeA(pkA, seed, received);

        short[] a = new short[NHParamsPredictable.N];
        generateA(a, seed);

        byte[] noiseSeed = new byte[32];
        rand.getRandom().nextBytes(noiseSeed);

        short[] sp = new short[NHParamsPredictable.N];
        NHPolyPredictable.getNoise(sp, noiseSeed, (byte)0);
        NHPolyPredictable.toNTT(sp);

        short[] ep = new short[NHParamsPredictable.N];
        NHPolyPredictable.getNoise(ep, noiseSeed, (byte)1);
        NHPolyPredictable.toNTT(ep);

        short[] bp = new short[NHParamsPredictable.N];
        NHPolyPredictable.pointWise(a, sp, bp);
        NHPolyPredictable.add(bp, ep, bp);

        short[] v = new short[NHParamsPredictable.N];
        NHPolyPredictable.pointWise(pkA, sp, v);
        NHPolyPredictable.fromNTT(v);

        short[] epp = new short[NHParamsPredictable.N];
        NHPolyPredictable.getNoise(epp, noiseSeed, (byte)2);
        NHPolyPredictable.add(v, epp, v);

        short[] c = new short[NHParamsPredictable.N];
        NHErrorCorrectionPredictable.helpRec(c, v, noiseSeed, (byte)3);

        encodeB(send, bp, c);

        NHErrorCorrectionPredictable.rec(sharedKey, v, c);

        if (!STATISTICAL_TEST)
        {
            sha3(sharedKey);
        }
    }

    public static void sharedA(byte[] sharedKey, short[] sk, byte[] received)
    {
        short[] bp = new short[NHParamsPredictable.N];
        short[] c = new short[NHParamsPredictable.N];
        decodeB(bp, c, received);

        short[] v = new short[NHParamsPredictable.N];
        NHPolyPredictable.pointWise(sk, bp, v);
        NHPolyPredictable.fromNTT(v);

        NHErrorCorrectionPredictable.rec(sharedKey, v, c);

        if (!STATISTICAL_TEST)
        {
            sha3(sharedKey);
        }
    }

    static void decodeA(short[] pk, byte[] seed, byte[] r)
    {
        NHPolyPredictable.fromBytes(pk, r);
        System.arraycopy(r, NHParamsPredictable.POLY_BYTES, seed, 0, NHParamsPredictable.SEED_BYTES);
    }

    static void decodeB(short[] b, short[] c, byte[] r)
    {
        NHPolyPredictable.fromBytes(b, r);

        for (int i = 0; i < NHParamsPredictable.N / 4; ++i)
        {
            int j = 4 * i;
            int ri = r[NHParamsPredictable.POLY_BYTES + i] & 0xFF;
            c[j + 0] = (short)(ri & 0x03);
            c[j + 1] = (short)((ri >>> 2) & 0x03);
            c[j + 2] = (short)((ri >>> 4) & 0x03);
            c[j + 3] = (short)(ri >>> 6);
        }
    }

    static void encodeA(byte[] r, short[] pk, byte[] seed)
    {
        NHPolyPredictable.toBytes(r, pk);
        System.arraycopy(seed, 0, r, NHParamsPredictable.POLY_BYTES, NHParamsPredictable.SEED_BYTES);
    }

    static void encodeB(byte[] r, short[] b, short[] c)
    {
        NHPolyPredictable.toBytes(r, b);

        for (int i = 0; i < NHParamsPredictable.N / 4; ++i)
        {
            int j = 4 * i;
            r[NHParamsPredictable.POLY_BYTES + i] = (byte)(c[j] | (c[j + 1] << 2) | (c[j + 2] << 4) | (c[j + 3] << 6));
        }
    }

    static void generateA(short[] a, byte[] seed)
    {
        NHPolyPredictable.uniform(a, seed);
    }

    static void sha3(byte[] sharedKey)
    {
        SHA3Digest d = new SHA3Digest(256);
        d.update(sharedKey, 0, 32);
        d.doFinal(sharedKey, 0);
    }
}
