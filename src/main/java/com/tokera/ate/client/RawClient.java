package com.tokera.ate.client;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.fs.FsFolderDto;
import com.tokera.ate.providers.YamlProvider;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.jboss.resteasy.client.jaxrs.ResteasyClient;
import org.jboss.resteasy.client.jaxrs.ResteasyClientBuilder;
import org.jboss.resteasy.client.jaxrs.ResteasyWebTarget;
import org.jboss.resteasy.plugins.providers.jackson.ResteasyJackson2Provider;

import javax.ws.rs.client.Entity;
import javax.ws.rs.client.Invocation;
import javax.ws.rs.core.MediaType;
import javax.ws.rs.core.MultivaluedMap;
import javax.ws.rs.core.Response;

public class RawClient {

    private ResteasyClient client;
    private String urlBase;
    private String prefixForRest;
    private String prefixForFs;
    @Nullable
    private String session = null;

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

    public String getSession() {
        return this.session;
    }

    public static ResteasyClient createResteasyClient() {
        ResteasyClient client = new ResteasyClientBuilder()
                .register(new YamlProvider())
                .register(new ResteasyJackson2Provider())
                .build();
        return client;
    }

    private ResteasyWebTarget target(String prefix, String postfix) {
        return client.target(urlBase + prefix + postfix);
    }

    public FsFolderDto fsList(String path) {
        String uri = path;
        Invocation.Builder builder = target(prefixForFs, uri)
                .request()
                .accept(MediaType.APPLICATION_JSON_TYPE);
        if (this.session != null) {
            builder = builder.header("Authorization", this.session);
        }
        Response response = builder.get();
        TestTools.validateResponse(response, uri);
        return response.readEntity(FsFolderDto.class);
    }

    public String fsGet(String path) {
        String uri = path;
        Invocation.Builder builder = target(prefixForFs, uri)
                .request()
                .accept(MediaType.WILDCARD);
        if (this.session != null) {
            builder = builder.header("Authorization", this.session);
        }
        Response response = builder.get();
        TestTools.validateResponse(response, uri);
        return response.readEntity(String.class);
    }

    public @Nullable String fsGetOrNull(String path) {
        Invocation.Builder builder = target(prefixForFs, path)
                .request()
                .accept(MediaType.WILDCARD);
        if (this.session != null) {
            builder = builder.header("Authorization", this.session);
        }
        Response response = builder.get();
        if (response.getStatus() < 200 || response.getStatus() >= 300) {
            return null;
        }
        return response.readEntity(String.class);
    }

    public String fsPost(String path, String data, MediaType mediaType) {
        return fsPost(path, Entity.text(data), mediaType);
    }

    public String fsPost(String path, Entity<?> data, MediaType mediaType) {
        Invocation.Builder builder = target(prefixForFs, path)
                .request(mediaType)
                .accept(MediaType.WILDCARD);
        if (this.session != null) {
            builder = builder.header("Authorization", this.session);
        }
        Response response = builder.post(data);
        TestTools.validateResponse(response, path);
        return response.readEntity(String.class);
    }

    public String fsPost(String path, String data, String mediaType) {
        return fsPost(path, Entity.text(data), mediaType);
    }

    public String fsPost(String path, Entity<?> data, String mediaType) {
        Invocation.Builder builder = target(prefixForFs, path)
                .request(mediaType)
                .accept(MediaType.WILDCARD);
        if (this.session != null) {
            builder = builder.header("Authorization", this.session);
        }
        Response response = builder.post(data);
        TestTools.validateResponse(response, path);
        return response.readEntity(String.class);
    }

    public String fsPut(String path, String data, MediaType mediaType) {
        return fsPut(path, Entity.entity(data, mediaType), mediaType);
    }

    public String fsPut(String path, Entity<?> data, MediaType mediaType) {
        Invocation.Builder builder = target(prefixForFs, path)
                .request(mediaType)
                .accept(MediaType.WILDCARD);
        if (this.session != null) {
            builder = builder.header("Authorization", this.session);
        }
        Response response = builder.put(data);
        TestTools.validateResponse(response, path);
        return response.readEntity(String.class);
    }

    public String fsPut(String path, String data, String mediaType) {
        return fsPut(path, Entity.entity(data, mediaType), mediaType);
    }

    public String fsPut(String path, Entity<?> data, String mediaType) {
        Invocation.Builder builder = target(prefixForFs, path)
                .request(mediaType)
                .accept(MediaType.WILDCARD);
        if (this.session != null) {
            builder = builder.header("Authorization", this.session);
        }
        Response response = builder.put(data);
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

    public <T> T restPut(String path, Entity<?> entity, Class<T> clazz) {
        return TestTools.restPut(this.session, buildUrl(path), entity).readEntity(clazz);
    }

    public <T> T restPut(String path, Entity<?> entity, Class<T> clazz, MultivaluedMap<String, Object> queryParams) {
        return TestTools.restPut(this.session, buildUrl(path), entity, queryParams).readEntity(clazz);
    }

    public <T> T restPost(String path, Entity<?> entity, Class<T> clazz) {
        return TestTools.restPost(this.session, buildUrl(path), entity).readEntity(clazz);
    }

    public <T> T restPost(String path, Entity<?> entity, Class<T> clazz, MultivaluedMap<String, Object> queryParams) {
        return TestTools.restPost(this.session, buildUrl(path), entity, queryParams).readEntity(clazz);
    }

    public <T> T restGet(String path, Class<T> clazz) {
        return TestTools.restGet(this.session, buildUrl(path)).readEntity(clazz);
    }

    public <T> T restGet(String path, Class<T> clazz, MultivaluedMap<String, Object> queryParams) {
        return TestTools.restGet(this.session, buildUrl(path), queryParams).readEntity(clazz);
    }

    public void restDelete(String path) {
        TestTools.restDelete(this.session, buildUrl(path));
    }

    public void restDelete(String path, MultivaluedMap<String, Object> queryParams) {
        TestTools.restDelete(this.session, buildUrl(path), queryParams);
    }

    public <T> @Nullable T restGetOrNull(String path, Class<T> clazz) {
        Response resp = TestTools.restGetOrNull(this.session, buildUrl(path));
        if (resp == null) return null;
        if (resp.getLength() <= 0) return null;
        return resp.readEntity(clazz);
    }

    public <T> T restGetAndOutput(String path, Class<T> clazz) {
        return TestTools.restGetAndOutput(this.session, buildUrl(path), clazz);
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
