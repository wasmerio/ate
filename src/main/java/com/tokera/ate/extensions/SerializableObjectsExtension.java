package com.tokera.ate.extensions;

import com.jsoniter.spi.TypeLiteral;
import com.tokera.ate.annotations.PermitParentFree;
import com.tokera.ate.annotations.PermitParentType;
import com.tokera.ate.annotations.YamlTag;

import javax.enterprise.event.Observes;
import javax.enterprise.inject.spi.Extension;
import javax.enterprise.inject.spi.ProcessAnnotatedType;
import javax.enterprise.inject.spi.WithAnnotations;
import java.util.ArrayList;

public class SerializableObjectsExtension implements Extension {
    private ArrayList<TypeLiteral> types = new ArrayList<>();

    public TypeLiteral[] asTypeLiterals() {
        return types.toArray(new TypeLiteral[types.size()]);
    }

    public void watchForDto(@Observes @WithAnnotations({PermitParentType.class, PermitParentFree.class, YamlTag.class}) ProcessAnnotatedType processAnnotatedType) {
        Class<?> resource = processAnnotatedType.getAnnotatedType().getJavaClass();
        types.add(TypeLiteral.create(resource));
    }
}
