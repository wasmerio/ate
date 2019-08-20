package com.tokera.ate.client;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.fs.FsFolderDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.providers.*;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.jboss.resteasy.client.jaxrs.ResteasyClient;
import org.jboss.resteasy.client.jaxrs.ResteasyClientBuilder;
import org.jboss.resteasy.client.jaxrs.ResteasyWebTarget;

import javax.ws.rs.client.Entity;
import javax.ws.rs.client.Invocation;
import javax.ws.rs.core.MediaType;
import javax.ws.rs.core.MultivaluedHashMap;
import javax.ws.rs.core.MultivaluedMap;
import javax.ws.rs.core.Response;
import java.util.List;
import java.util.Map;

public class RawClient {

    private ResteasyClient client;
    private String urlBase;
    private String prefixForRest;
    private String prefixForFs;
    @Nullable
    private String session = null;
    @Nullable
    private IPartitionKey partitionKey = null;
    private MultivaluedMap<String, Object> headers = new MultivaluedHashMap<>();

    public RawClient(String urlBase, @Nullable String session, String prefixForRest, String prefixForFs) {
        this.urlBase = urlBase;
        this.client = RawClient.createResteasyClient();
        this.session = session;
        this.prefixForRest = prefixForRest;
        this.prefixForFs = prefixForFs;
    }

    public RawClient setPrefixForRest(String prefix) {
        this.prefixForRest = prefix;
        return this;
    }

    public RawClient setPrefixForFs(String prefix) {
        this.prefixForFs = prefix;
        return this;
    }

    public RawClient appendToPrefixForRest(String prefix) {
        this.prefixForRest += prefix;
        return this;
    }

    public RawClient appendToPrefixForFs(String prefix) {
        this.prefixForFs += prefix;
        return this;
    }

    public void addHeader(String header, Object val) {
        this.headers.add(header, val);
    }

    public String getSession() {
        return this.session;
    }

    public void setSession(String val) { this.session = val; }

    public IPartitionKey getPartitionKey() { return this.partitionKey; }

    public void setPartitionKey(IPartitionKey val) { this.partitionKey = val; }

    public static ResteasyClient createResteasyClient() {
        ResteasyClient client = new ResteasyClientBuilder()
                .register(new YamlProvider())
                .register(new UuidSerializer())
                .register(new GenericPartitionKeySerializer())
                .register(new PartitionKeySerializer())
                .register(new PuuidSerializer())
                .register(new TokenSerializer())
                .register(new PrivateKeyWithSeedSerializer())
                .register(new CountLongSerializer())
                .register(new RangeLongSerializer())
                .build();
        return client;
    }

    private Invocation.Builder addHeaders(Invocation.Builder builder) {
        if (this.session != null) {
            builder = builder.header("Authorization", this.session);
        }
        if (this.partitionKey != null) {
            builder = builder.header("PartitionKey", PartitionKeySerializer.serialize(this.partitionKey));
        }
        for (Map.Entry<String, List<Object>> entry : this.headers.entrySet()) {
            for (Object headerVal : entry.getValue()) {
                builder = builder.header(entry.getKey(), headerVal);
            }
        }
        return builder;
    }

    private Invocation.Builder targetRelative(String prefix, String postfix, MediaType accepts) {
        return target(urlBase + prefix + postfix, null, accepts, null);
    }

    private Invocation.Builder targetRelative(String prefix, String postfix, MediaType accepts, MediaType requestMedia) {
        return target(urlBase + prefix + postfix, null, accepts, requestMedia);
    }

    private Invocation.Builder target(String uri, @Nullable MultivaluedMap<String, Object> queryParams, MediaType accepts, @Nullable MediaType requestMedia) {
        ResteasyWebTarget webTarget = client.target(uri);

        if (queryParams != null) {
            webTarget = webTarget.queryParams(queryParams);
        }

        Invocation.Builder builder;
        if (requestMedia != null) {
            builder = webTarget.request(requestMedia);
        } else {
            builder = webTarget.request();
        }

        builder = builder.accept(accepts);
        builder = addHeaders(builder);
        return builder;
    }

    public FsFolderDto fsList(String path) {
        Response response = targetRelative(prefixForFs, path, MediaType.APPLICATION_JSON_TYPE).get();
        TestTools.validateResponse(response, path);
        return response.readEntity(FsFolderDto.class);
    }

    public String fsGet(String path) {
        Response response = targetRelative(prefixForFs, path, MediaType.WILDCARD_TYPE).get();
        TestTools.validateResponse(response, path);
        return response.readEntity(String.class);
    }

    public @Nullable String fsGetOrNull(String path) {
        Response response = targetRelative(prefixForFs, path, MediaType.WILDCARD_TYPE).get();
        if (response.getStatus() < 200 || response.getStatus() >= 300) {
            return null;
        }
        return response.readEntity(String.class);
    }

    public String fsPost(String path, String data, MediaType mediaType) {
        return fsPost(path, Entity.text(data), mediaType);
    }

    public String fsPost(String path, Entity<?> data, MediaType mediaType) {
        Response response = targetRelative(prefixForFs, path, MediaType.WILDCARD_TYPE, mediaType).post(data);
        TestTools.validateResponse(response, path);
        return response.readEntity(String.class);
    }

    public String fsPut(String path, String data, MediaType mediaType) {
        return fsPut(path, Entity.entity(data, mediaType), mediaType);
    }

    public String fsPut(String path, Entity<?> data, MediaType mediaType) {
        Response response = targetRelative(prefixForFs, path, MediaType.WILDCARD_TYPE, mediaType).put(data);
        TestTools.validateResponse(response, path);
        return response.readEntity(String.class);
    }

    private String buildUrl(String path) {
        if (path.startsWith("/") == false && prefixForRest.endsWith("/") == false) {
            return this.urlBase + prefixForRest + "/" + path;
        } else {
            return this.urlBase + prefixForRest + path;
        }
    }

    public <T> T restGet(String path, Class<T> clazz) {
        String url = buildUrl(path);
        return TestTools.restRunner(() -> target(url, null, MediaType.WILDCARD_TYPE, MediaType.WILDCARD_TYPE)
                .get(), url)
                .readEntity(clazz);
    }

    public <T> T restGet(String path, Class<T> clazz, MultivaluedMap<String, Object> queryParams) {
        String url = buildUrl(path);
        return TestTools.restRunner(() -> target(url, queryParams, MediaType.WILDCARD_TYPE, MediaType.WILDCARD_TYPE)
                .get(), url)
                .readEntity(clazz);
    }

    public <T> T restPut(String path, Entity<?> entity, Class<T> clazz) {
        String url = buildUrl(path);
        return TestTools.restRunner(() -> target(url, null, MediaType.WILDCARD_TYPE, MediaType.WILDCARD_TYPE)
                .put(entity), url)
                .readEntity(clazz);
    }

    public <T> T restPut(String path, Entity<?> entity, Class<T> clazz, MultivaluedMap<String, Object> queryParams) {
        String url = buildUrl(path);
        return TestTools.restRunner(() -> target(url, queryParams, MediaType.WILDCARD_TYPE, MediaType.WILDCARD_TYPE)
                .put(entity), url)
                .readEntity(clazz);
    }

    public <T> T restPost(String path, Entity<?> entity, Class<T> clazz) {
        String url = buildUrl(path);
        return TestTools.restRunner(() -> target(url, null, MediaType.WILDCARD_TYPE, MediaType.WILDCARD_TYPE)
                .post(entity), url)
                .readEntity(clazz);
    }

    public <T> T restPost(String path, Entity<?> entity, Class<T> clazz, MultivaluedMap<String, Object> queryParams) {
        String url = buildUrl(path);
        return TestTools.restRunner(() -> target(url, queryParams, MediaType.WILDCARD_TYPE, MediaType.WILDCARD_TYPE)
                .post(entity), url)
                .readEntity(clazz);
    }

    public void restDelete(String path) {
        String url = buildUrl(path);
        TestTools.restRunner(() -> target(url, null, MediaType.WILDCARD_TYPE, MediaType.WILDCARD_TYPE)
                .delete(), url);
    }

    public void restDelete(String path, MultivaluedMap<String, Object> queryParams) {
        String url = buildUrl(path);
        TestTools.restRunner(() -> target(url, queryParams, MediaType.WILDCARD_TYPE, MediaType.WILDCARD_TYPE)
                .delete(), url);
    }

    public <T> @Nullable T restGetOrNull(String path, Class<T> clazz) {
        String url = buildUrl(path);
        Response resp = TestTools.restRunner(() -> target(url, null, MediaType.WILDCARD_TYPE, MediaType.WILDCARD_TYPE)
                .get(), url);
        if (resp == null) return null;
        if (resp.getLength() <= 0) return null;
        return resp.readEntity(clazz);
    }

    public <T> T restGetAndOutput(String path, Class<T> clazz) {
        T ret = restGet(path, clazz);
        System.out.println(AteDelegate.get().yaml.serializeObj(ret));
        return ret;
    }

    public static RawClient createViaRestPost(String server, Integer port, String prefixForRest, String path, Entity<?> entity) {
        String url = RawClientBuilder.generateServerUrl(true, server, port) + prefixForRest + path;

        AteDelegate d = AteDelegate.get();
        Response response = TestTools.restPost(null, url, entity);

        String auth = response.getHeaderString("Authorization");
        d.genericLogger.info("auth:\n" + auth);

        return new RawClientBuilder()
                .withSession(auth)
                .server(server)
                .port(port)
                .prefixForRest(prefixForRest)
                .build();
    }
}
