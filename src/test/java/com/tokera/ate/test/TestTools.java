package com.tokera.ate.test;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.providers.YamlProvider;
import org.apache.commons.io.IOUtils;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.jboss.resteasy.client.jaxrs.ResteasyClient;
import org.jboss.resteasy.client.jaxrs.ResteasyClientBuilder;
import org.jboss.resteasy.plugins.providers.jackson.ResteasyJackson2Provider;
import org.junit.jupiter.api.Assertions;

import javax.ws.rs.WebApplicationException;
import java.io.IOException;
import java.io.InputStream;
import java.util.ArrayList;
import java.util.List;

public class TestTools {

    public static ResteasyClient buildClient() {
        ResteasyClient client = new ResteasyClientBuilder()
                .register(new YamlProvider())
                .register(new ResteasyJackson2Provider())
                .build();
        return client;
    }

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
                throw new RuntimeException("Unsupported array comparison");
            }

        } else {
            Assertions.assertEquals(obj1, obj2);
        }
    }

    private static List<MessagePrivateKeyDto> getKeys(String name) {
        List<MessagePrivateKeyDto> ret = new ArrayList<>();

        InputStream inputStream = ClassLoader.getSystemResourceAsStream("keys/" + name);
        assert inputStream != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(inputStream);

        try {
            String keysFile = IOUtils.toString(inputStream, com.google.common.base.Charsets.UTF_8);

            for (String _keyTxt : keysFile.split("\\.\\.\\.")) {
                String keyTxt = _keyTxt + "...";

                Object obj = AteDelegate.get().yaml.deserializeObj(keyTxt);
                if (obj instanceof MessagePrivateKeyDto) {
                    MessagePrivateKeyDto key = (MessagePrivateKeyDto) obj;
                    ret.add(key);
                }
            }

        } catch (IOException e) {
            throw new WebApplicationException(e);
        }

        return ret;
    }

    public static void initSeedKeys() {
        AteDelegate d = AteDelegate.get();
        for (MessagePrivateKeyDto key : getKeys("sign.keys.64")) {
            d.encryptor.addSeedKeySign64(key);
        }
        for (MessagePrivateKeyDto key : getKeys("sign.keys.128")) {
            d.encryptor.addSeedKeySign128(key);
        }
        for (MessagePrivateKeyDto key : getKeys("sign.keys.256")) {
            d.encryptor.addSeedKeySign256(key);
        }
        for (MessagePrivateKeyDto key : getKeys("encrypt.keys.128")) {
            d.encryptor.addSeedKeyEncrypt128(key);
        }
        for (MessagePrivateKeyDto key : getKeys("encrypt.keys.256")) {
            d.encryptor.addSeedKeyEncrypt256(key);
        }
    }
}
