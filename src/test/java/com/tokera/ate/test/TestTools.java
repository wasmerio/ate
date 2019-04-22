package com.tokera.ate.test;

import org.checkerframework.checker.nullness.qual.Nullable;
import org.junit.jupiter.api.Assertions;

import javax.ws.rs.WebApplicationException;

public class TestTools {


    public static void assertEqualAndNotNull(@Nullable Object _obj1, @Nullable Object _obj2) {
        Object obj1 = _obj1;
        Object obj2 = _obj2;

        assert obj1 != null : "@AssumeAssertion(nullness): Must not be null";
        assert obj2 != null : "@AssumeAssertion(nullness): Must not be null";

        Assertions.assertNotNull(obj1);
        Assertions.assertNotNull(obj2);
        Assertions.assertEquals(obj1.getClass(), obj2.getClass());

        if (obj1.getClass().isArray()) {
            if (obj1 instanceof int[]) {
                Assertions.assertArrayEquals((int[]) obj1, (int[]) obj2);
            } else if (obj1 instanceof byte[]) {
                Assertions.assertArrayEquals((byte[]) obj1, (byte[]) obj2);
            } else if (obj1 instanceof char[]) {
                Assertions.assertArrayEquals((char[]) obj1, (char[]) obj2);
            } else if (obj1 instanceof long[]) {
                Assertions.assertArrayEquals((long[]) obj1, (long[]) obj2);
            } else if (obj1 instanceof float[]) {
                Assertions.assertArrayEquals((float[]) obj1, (float[]) obj2);
            } else if (obj1 instanceof short[]) {
                Assertions.assertArrayEquals((short[]) obj1, (short[]) obj2);
            } else if (obj1 instanceof double[]) {
                Assertions.assertArrayEquals((double[]) obj1, (double[]) obj2);
            } else if (obj1 instanceof boolean[]) {
                Assertions.assertArrayEquals((boolean[]) obj1, (boolean[]) obj2);
            } else {
                throw new WebApplicationException("Unsupported array comparison");
            }

        } else {
            Assertions.assertEquals(obj1, obj2);
        }
    }
}
