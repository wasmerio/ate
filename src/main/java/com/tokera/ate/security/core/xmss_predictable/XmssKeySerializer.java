package com.tokera.ate.security.core.xmss_predictable;

import org.bouncycastle.crypto.digests.SHA512Digest;
import org.bouncycastle.pqc.crypto.xmss.XMSSMTParameters;
import org.bouncycastle.pqc.crypto.xmss.XMSSMTPrivateKeyParameters;
import org.bouncycastle.pqc.crypto.xmss.XMSSMTPublicKeyParameters;

import java.io.ByteArrayOutputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.nio.ByteBuffer;

public class XmssKeySerializer {
    private static void writeBytes(DataOutputStream dos, byte[] data) throws IOException {
        dos.writeInt(data.length);
        dos.write(data);
    }

    private static byte[] readBytes(ByteBuffer bb) {
        int len = bb.getInt();
        if (len <= 0) return new byte[0];
        byte[] ret = new byte[len];
        bb.get(ret);
        return ret;
    }

    public static byte[] serialize(XMSSMTPublicKeyParameters params) {
        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        DataOutputStream dos = new DataOutputStream(stream);

        try {
            XMSSMTParameters innerParams = params.getParameters();
            dos.writeInt(innerParams.getHeight());
            dos.writeInt(innerParams.getLayers());

            writeBytes(dos, params.getPublicSeed());
            writeBytes(dos, params.getRoot());

            return stream.toByteArray();
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
    }

    public static XMSSMTPublicKeyParameters deserializePublic(byte[] data) {
        ByteBuffer bb = ByteBuffer.wrap(data);
        int height = bb.getInt();
        int layers = bb.getInt();
        XMSSMTParameters params = new XMSSMTParameters(height, layers, new SHA512Digest());

        byte[] publicSeed = readBytes(bb);
        byte[] root = readBytes(bb);

        return new XMSSMTPublicKeyParameters.Builder(params)
                .withPublicSeed(publicSeed)
                .withRoot(root)
                .build();
    }

    public static byte[] serialize(XMSSMTPrivateKeyParameters params) {
        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        DataOutputStream dos = new DataOutputStream(stream);

        try {
            XMSSMTParameters innerParams = params.getParameters();
            dos.writeInt(innerParams.getHeight());
            dos.writeInt(innerParams.getLayers());

            dos.writeLong(params.getIndex());
            writeBytes(dos, params.getSecretKeySeed());
            writeBytes(dos, params.getSecretKeyPRF());
            writeBytes(dos, params.getPublicSeed());
            writeBytes(dos, params.getRoot());

            return stream.toByteArray();
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
    }

    public static XMSSMTPrivateKeyParameters deserializePrivate(byte[] data) {
        ByteBuffer bb = ByteBuffer.wrap(data);
        int height = bb.getInt();
        int layers = bb.getInt();
        XMSSMTParameters params = new XMSSMTParameters(height, layers, new SHA512Digest());

        long index = bb.getLong();
        byte[] secretKeySeed = readBytes(bb);
        byte[] secretKeyPRF = readBytes(bb);
        byte[] publicSeed = readBytes(bb);
        byte[] root = readBytes(bb);

        return new XMSSMTPrivateKeyParameters.Builder(params)
                .withIndex(index)
                .withSecretKeySeed(secretKeySeed)
                .withSecretKeyPRF(secretKeyPRF)
                .withPublicSeed(publicSeed)
                .withRoot(root)
                .build();
    }
}
