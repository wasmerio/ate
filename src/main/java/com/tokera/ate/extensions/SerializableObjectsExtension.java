package com.tokera.ate.extensions;

import com.tokera.ate.annotations.PermitParentFree;
import com.tokera.ate.annotations.PermitParentType;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.common.ImmutalizableArrayList;
import com.tokera.ate.common.ImmutalizableHashMap;
import com.tokera.ate.common.MapTools;
import org.jboss.weld.environment.se.events.ContainerInitialized;

import javax.enterprise.event.Observes;
import javax.enterprise.inject.spi.Extension;
import javax.enterprise.inject.spi.ProcessAnnotatedType;
import javax.enterprise.inject.spi.WithAnnotations;
import java.util.Map;

public class SerializableObjectsExtension implements Extension {
    private final ImmutalizableHashMap<String, Class<?>> lookup = new ImmutalizableHashMap<>();

    public Map<String, Class<?>> asMap() {
        return lookup;
    }

    @SuppressWarnings({"unchecked"})
    public void watchForDto(@Observes @WithAnnotations({PermitParentType.class, PermitParentFree.class, YamlTag.class}) ProcessAnnotatedType processAnnotatedType) {
        Class<?> resource = processAnnotatedType.getAnnotatedType().getJavaClass();

        lookup.put(resource.getName(), resource);
    }

    public void start(@Observes final ContainerInitialized event) {
        lookup.immutalize();
    }

    @SuppressWarnings({"unchecked"})
    public Class<?> findClass(String clazzName) {
        Class<?> ret = MapTools.getOrNull(lookup, clazzName);
        if (ret != null) return ret;
        try {
            return Class.forName(clazzName);
        } catch (ClassNotFoundException e) {
            throw new RuntimeException(e);
        }
    }

    @SuppressWarnings({"unchecked"})
    public <T> Class<T> findClass(String clazzName, Class<T> baseClass) {
        Class<T> ret = (Class<T>)MapTools.getOrNull(lookup, clazzName);
        if (ret != null) return ret;
        try {
            return (Class<T>)Class.forName(clazzName);
        } catch (ClassNotFoundException e) {
            throw new RuntimeException(e);
        }
    }
}
