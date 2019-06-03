package com.tokera.ate.security.core;

import org.bouncycastle.crypto.digests.SHA512Digest;
import org.bouncycastle.pqc.crypto.rainbow.Layer;
import org.bouncycastle.pqc.crypto.rainbow.RainbowPrivateKeyParameters;
import org.bouncycastle.pqc.crypto.rainbow.RainbowPublicKeyParameters;
import org.bouncycastle.pqc.crypto.xmss.XMSSMTParameters;
import org.bouncycastle.pqc.crypto.xmss.XMSSMTPrivateKeyParameters;
import org.bouncycastle.pqc.crypto.xmss.XMSSMTPublicKeyParameters;

import java.io.ByteArrayOutputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.nio.ByteBuffer;

public class RainbowKeySerializer {
    private static void writeLayerArray(DataOutputStream dos, Layer[] data) throws IOException {
        dos.writeInt(data.length);
        for (int n = 0; n < data.length; n++) {
            Layer layer = data[n];
            dos.writeByte(layer.getVi());
            dos.writeByte(layer.getViNext());
            writeShortTripleArray(dos, layer.getCoeffAlpha());
            writeShortTripleArray(dos, layer.getCoeffBeta());
            writeShortDoubleArray(dos, layer.getCoeffGamma());
            writeShortArray(dos, layer.getCoeffEta());
        }
    }

    private static Layer[] readLayerArray(ByteBuffer bb) {
        int len = bb.getInt();
        if (len <= 0) return new Layer[0];
        Layer[] ret = new Layer[len];
        for (int n = 0; n < len; n++) {
            Layer layer = new Layer(
                    bb.get(),
                    bb.get(),
                    readShortTripleArray(bb),
                    readShortTripleArray(bb),
                    readShortDoubleArray(bb),
                    readShortArray(bb)
            );
            ret[n] = layer;
        }

        return ret;
    }
    private static void writeIntArray(DataOutputStream dos, int[] data) throws IOException {
        dos.writeInt(data.length);
        for (int n = 0; n < data.length; n++) {
            dos.writeInt(data[n]);
        }
    }

    private static int[] readIntArray(ByteBuffer bb) {
        int len = bb.getInt();
        if (len <= 0) return new int[0];
        int[] ret = new int[len];
        for (int n = 0; n < len; n++) {
            ret[n] = bb.getInt();
        }

        return ret;
    }
    private static void writeShortArray(DataOutputStream dos, short[] data) throws IOException {
        dos.writeInt(data.length);
        for (int n = 0; n < data.length; n++) {
            dos.writeShort(data[n]);
        }
    }

    private static short[] readShortArray(ByteBuffer bb) {
        int len = bb.getInt();
        if (len <= 0) return new short[0];
        short[] ret = new short[len];
        for (int n = 0; n < len; n++) {
            ret[n] = bb.getShort();
        }

        return ret;
    }

    private static void writeShortDoubleArray(DataOutputStream dos, short[][] data) throws IOException {
        dos.writeInt(data.length);
        for (int n = 0; n < data.length; n++) {
            writeShortArray(dos, data[n]);
        }
    }

    private static short[][] readShortDoubleArray(ByteBuffer bb) {
        int len = bb.getInt();
        if (len <= 0) return new short[0][];
        short[][] ret = new short[len][];
        for (int n = 0; n < len; n++) {
            ret[n] = readShortArray(bb);
        }

        return ret;
    }

    private static void writeShortTripleArray(DataOutputStream dos, short[][][] data) throws IOException {
        dos.writeInt(data.length);
        for (int n = 0; n < data.length; n++) {
            writeShortDoubleArray(dos, data[n]);
        }
    }

    private static short[][][] readShortTripleArray(ByteBuffer bb) {
        int len = bb.getInt();
        if (len <= 0) return new short[0][][];
        short[][][] ret = new short[len][][];
        for (int n = 0; n < len; n++) {
            ret[n] = readShortDoubleArray(bb);
        }

        return ret;
    }

    public static byte[] serialize(RainbowPublicKeyParameters params) {
        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        DataOutputStream dos = new DataOutputStream(stream);

        try {
            dos.writeInt(params.getDocLength());
            writeShortDoubleArray(dos, params.getCoeffQuadratic());
            writeShortDoubleArray(dos, params.getCoeffSingular());
            writeShortArray(dos, params.getCoeffScalar());
            return stream.toByteArray();
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
    }

    public static byte[] serialize(RainbowPrivateKeyParameters params) {
        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        DataOutputStream dos = new DataOutputStream(stream);

        try {
            writeShortDoubleArray(dos, params.getInvA1());
            writeShortArray(dos, params.getB1());
            writeShortDoubleArray(dos, params.getInvA2());
            writeShortArray(dos, params.getB2());
            writeIntArray(dos, params.getVi());
            writeLayerArray(dos, params.getLayers());
            return stream.toByteArray();
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
    }

    public static RainbowPublicKeyParameters deserializePublic(byte[] data) {
        ByteBuffer bb = ByteBuffer.wrap(data);
        return new RainbowPublicKeyParameters(
                bb.getInt(),
                readShortDoubleArray(bb),
                readShortDoubleArray(bb),
                readShortArray(bb)
        );
    }

    public static RainbowPrivateKeyParameters deserializePrivate(byte[] data) {
        ByteBuffer bb = ByteBuffer.wrap(data);
        return new RainbowPrivateKeyParameters(
                readShortDoubleArray(bb),
                readShortArray(bb),
                readShortDoubleArray(bb),
                readShortArray(bb),
                readIntArray(bb),
                readLayerArray(bb)
        );
    }
}
