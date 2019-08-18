package com.tokera.ate.delegates;

import com.tokera.ate.scopes.Startup;
import org.apache.commons.io.IOUtils;
import org.junit.jupiter.api.Assertions;
import org.reflections.Reflections;
import org.reflections.scanners.ResourcesScanner;
import org.reflections.util.ClasspathHelper;
import org.reflections.util.ConfigurationBuilder;
import org.reflections.util.FilterBuilder;

import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.Response;

import java.io.IOException;
import java.io.InputStream;
import java.util.ArrayList;
import java.util.List;

import static org.reflections.util.Utils.findLogger;

@Startup
@ApplicationScoped
public class ResourceFileDelegate {
    AteDelegate d = AteDelegate.get();

    private Reflections resReflection;

    public ResourceFileDelegate() {
        Reflections.log = null;
        resReflection = new Reflections(
                new ConfigurationBuilder()
                        .filterInputsBy(new FilterBuilder()
                                .exclude("(.*)\\.so$"))
                        .setUrls(ClasspathHelper.forClassLoader())
                        .setScanners(new ResourcesScanner()));
        Reflections.log = findLogger(Reflections.class);
    }

    @SuppressWarnings("unchecked")
    public <T> List<T> loadAll(String prefix, Class<T> clazz) {
        List<T> ret = new ArrayList<>();

        for (String file : resReflection.getResources(n -> true)) {
            if (file.startsWith(prefix) == false)
                continue;

            ret.addAll(loadFile(file, clazz));
        }

        return ret;
    }

    @SuppressWarnings("unchecked")
    public <T> List<T> loadFile(String file, Class<T> clazz) {
        List<T> ret = new ArrayList<>();

        try {
            InputStream inputStream = ClassLoader.getSystemResourceAsStream(file);
            assert inputStream != null : "@AssumeAssertion(nullness): Must not be null";
            Assertions.assertNotNull(inputStream);

            String data = IOUtils.toString(inputStream, com.google.common.base.Charsets.UTF_8);

            if (file.endsWith("yml") || file.endsWith("yaml")) {
                for (String _keyTxt : data.split("\\.\\.\\.")) {
                    String keyTxt = _keyTxt + "...";

                    Object obj = AteDelegate.get().yaml.deserializeObj(keyTxt);
                    if (obj != null && obj.getClass() == clazz) {
                        ret.add((T)obj);
                    }
                }
            }
            if (file.endsWith("json")) {
                Object obj = AteDelegate.get().json.deserialize(data, clazz);
                if (obj != null && obj.getClass() == clazz) {
                    ret.add((T)obj);
                }
            }
        } catch (IOException ex) {
            throw new WebApplicationException("Failed to load file", ex, Response.Status.INTERNAL_SERVER_ERROR);
        }

        return ret;
    }
}
