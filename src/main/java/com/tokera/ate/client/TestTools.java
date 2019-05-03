package com.tokera.ate.client;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.providers.YamlProvider;
import org.apache.commons.io.IOUtils;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.jboss.resteasy.client.jaxrs.ResteasyClient;
import org.jboss.resteasy.client.jaxrs.ResteasyClientBuilder;
import org.jboss.resteasy.client.jaxrs.engines.ApacheHttpClient4Engine;
import org.jboss.resteasy.plugins.providers.jackson.ResteasyJackson2Provider;
import org.junit.jupiter.api.Assertions;

import javax.ws.rs.ClientErrorException;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.client.Entity;
import javax.ws.rs.client.Invocation;
import javax.ws.rs.core.MediaType;
import javax.ws.rs.core.Response;
import java.io.IOException;
import java.io.InputStream;
import java.util.ArrayList;
import java.util.List;

public class TestTools {

    public static ResteasyClient buildResteasyClient() {
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

    @SuppressWarnings("argument.type.incompatible")
    static public void assertEquals(String expected, @Nullable String actual) {
        Assertions.assertEquals(expected, actual);
    }

    private static List<MessagePrivateKeyDto> getTestKeys(String name) {
        List<MessagePrivateKeyDto> ret = new ArrayList<>();

        InputStream inputStream = ClassLoader.getSystemResourceAsStream("test-keys/" + name);
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

    public static void initSeedTestKeys() {
        AteDelegate d = AteDelegate.get();
        for (MessagePrivateKeyDto key : getTestKeys("sign.keys.64")) {
            d.encryptor.addSeedKeySign64(key);
        }
        for (MessagePrivateKeyDto key : getTestKeys("sign.keys.128")) {
            d.encryptor.addSeedKeySign128(key);
        }
        for (MessagePrivateKeyDto key : getTestKeys("sign.keys.256")) {
            d.encryptor.addSeedKeySign256(key);
        }
        for (MessagePrivateKeyDto key : getTestKeys("encrypt.keys.128")) {
            d.encryptor.addSeedKeyEncrypt128(key);
        }
        for (MessagePrivateKeyDto key : getTestKeys("encrypt.keys.256")) {
            d.encryptor.addSeedKeyEncrypt256(key);
        }
    }

    public static Response restPut(@Nullable String token, String url, Entity<?> entity) {
        Response resp;
        ResteasyClient client = TestTools.buildResteasyClient();
        try {
            Invocation.Builder target = client.target(url)
                    .request()
                    .accept(MediaType.WILDCARD_TYPE);
            if (token != null) {
                target = target.header("Authorization", token);
            }
            resp = target.put(entity);

        } catch (ClientErrorException e) {
            resp = e.getResponse();
            resp.close();

            validateResponse(resp, url);
            throw new WebApplicationException(e);
        }

        validateResponse(resp, url);
        return resp;
    }

    @SuppressWarnings("known.nonnull")
    public static void validateResponse(Response resp) {
        if (resp.getLocation() != null) {
            validateResponse(resp, resp.getLocation().toString());
        } else {
            validateResponse(resp, null);
        }
    }

    public static void validateResponse(Response resp, @Nullable String uri) {
        if (resp.getStatus() < 200 || resp.getStatus() >= 300) {
            String urlTxt = "";
            if (uri != null) {
                urlTxt = " while processing URL:[" + uri + "]";
            }

            String entity = resp.readEntity(String.class).replace("\r", "\n");
            if (entity.length() > 0) entity = "\n" + entity;

            throw new WebApplicationException(resp.getStatusInfo().getReasonPhrase() + urlTxt + entity, resp.getStatus());
        }
    }

    public static Response restPost(@Nullable String token, String url, Entity<?> entity) {
        Response resp;
        ResteasyClient client = TestTools.buildResteasyClient();
        try {
            Invocation.Builder target = client.target(url)
                    .request()
                    .accept(MediaType.WILDCARD_TYPE);
            if (token != null) {
                target = target.header("Authorization", token);
            }
            resp = target.post(entity);

        } catch (ClientErrorException e) {
            resp = e.getResponse();
            resp.close();

            validateResponse(resp, url);
            throw new WebApplicationException(e);
        }

        validateResponse(resp, url);
        return resp;
    }

    public static Response restGet(@Nullable String token, String url) {
        Response resp;
        ResteasyClient client = TestTools.buildResteasyClient();
        try {
            Invocation.Builder target = client.target(url)
                    .request()
                    .accept(MediaType.WILDCARD_TYPE);
            if (token != null) {
                target = target.header("Authorization", token);
            }
            resp = target.get();
        } catch (ClientErrorException e) {
            resp = e.getResponse();
            resp.close();

            validateResponse(resp, url);
            throw new WebApplicationException(e);
        }

        validateResponse(resp, url);
        return resp;
    }

    public static <T> T restGetAndOutput(@Nullable String token, String path, Class<T> clazz) {
        AteDelegate d = AteDelegate.get();

        Response response = restGet(token, path);
        T ret = response.readEntity(clazz);
        assert ret != null : "@AssumeAssertion(nullness): Must not be null";
        System.out.println(d.yaml.serializeObj(ret));
        return ret;
    }
}
