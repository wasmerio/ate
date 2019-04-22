package com.tokera.ate.extensions;

import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.dto.*;
import com.tokera.ate.dto.msg.*;

import java.lang.annotation.Annotation;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import javax.enterprise.event.Observes;
import javax.enterprise.inject.spi.Extension;
import javax.enterprise.inject.spi.ProcessAnnotatedType;
import javax.enterprise.inject.spi.WithAnnotations;

/**
 * Extension that discovers all the data objects marked with yaml tags
 */
public class YamlTagDiscoveryExtension implements Extension {

    private final Set<Class<?>> yamlTagClasses = new HashSet<>();
    private final Map<String, Class<?>> yamlLooked = new HashMap<>();
    private final Map<String, String> yamlReverse = new HashMap<>();

    @SuppressWarnings("method.invocation.invalid")
    public YamlTagDiscoveryExtension() {
        watchForYamlTag(MessageDataDigestDto.class);
        watchForYamlTag(MessageDataDto.class);
        watchForYamlTag(MessageDataHeaderDto.class);
        watchForYamlTag(MessageDataMetaDto.class);
        watchForYamlTag(MessageEncryptTextDto.class);
        watchForYamlTag(MessageMetaDto.class);
        watchForYamlTag(MessagePrivateKeyDto.class);
        watchForYamlTag(MessagePublicKeyDto.class);
        watchForYamlTag(MessageSyncDto.class);
        watchForYamlTag(ClaimDto.class);
        watchForYamlTag(TokenDto.class);
    }
    
    public void watchForYamlTag(@Observes @WithAnnotations(YamlTag.class) ProcessAnnotatedType processAnnotatedType) {
        Class<?> resource = processAnnotatedType.getAnnotatedType().getJavaClass();
        watchForYamlTag(resource);
    }
    
    public void watchForYamlTag(Class<?> resource) {
        
        Annotation[] anns = resource.getAnnotations();
        for (Object ann_ : anns) {
            if (ann_ instanceof YamlTag) {
                YamlTag ann = (YamlTag)ann_;

                yamlTagClasses.add(resource);
                yamlLooked.put(ann.value(), resource);

                String name = resource.getCanonicalName();
                if (name == null) continue;
                yamlReverse.put(name, ann.value());
            }
        }
    }
    
    public Set<Class<?>> getYamlTagClasses() {
        return this.yamlTagClasses;
    }
    
    public Map<String, Class<?>> getYamlTagLookup() {
        return this.yamlLooked;
    }

    public Map<String, String> getYamlReverse() {
        return this.yamlReverse;
    }
}
