package com.tokera.ate.common;

import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.NonNull;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.nio.ByteBuffer;

public final class ByteBufferTools {

    public static @Nullable String toBase64(@NonNull ByteBuffer bb) {
        if (bb.remaining() <= 0) {
            return null;
        }
        byte[] arr = new byte[bb.remaining()];
        bb.get(arr);
        return Base64.encodeBase64URLSafeString(arr);
    }
}
