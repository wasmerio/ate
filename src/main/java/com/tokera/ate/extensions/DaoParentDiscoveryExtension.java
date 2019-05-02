package com.tokera.ate.extensions;

import com.google.common.collect.HashMultimap;
import com.google.common.collect.Multimap;
import com.tokera.ate.annotations.PermitParentFree;
import com.tokera.ate.annotations.PermitParentType;

import java.lang.annotation.Annotation;
import java.util.HashSet;
import java.util.Set;
import javax.enterprise.event.Observes;
import javax.enterprise.inject.spi.BeforeBeanDiscovery;
import javax.enterprise.inject.spi.Extension;
import javax.enterprise.inject.spi.ProcessAnnotatedType;
import javax.enterprise.inject.spi.WithAnnotations;

/**
 * Extension that will build a graph of allowed parent/child relationships for all data objects
 */
public class DaoParentDiscoveryExtension implements Extension {

    private final HashSet<Class<?>> allowedParentFree = new HashSet<>();
    private final Multimap<Class<?>, Class<?>> allowedParents = HashMultimap.create();
    private final Multimap<Class<?>, Class<?>> allowedChildren = HashMultimap.create();
    private final HashSet<String> allowedParentFreeSimple = new HashSet<>();
    private final Multimap<String, String> allowedParentsSimple = HashMultimap.create();
    private final Multimap<String, String> allowedChildrenSimple = HashMultimap.create();

    public DaoParentDiscoveryExtension() {
    }
    
    public void watchForPermitParentType(@Observes @WithAnnotations(PermitParentType.class) ProcessAnnotatedType processAnnotatedType) {
        Class<?> resource = processAnnotatedType.getAnnotatedType().getJavaClass();
        watchForPermitParentType(resource);
    }

    public void watchForPermitParentFree(@Observes @WithAnnotations(PermitParentFree.class) ProcessAnnotatedType processAnnotatedType) {
        Class<?> resource = processAnnotatedType.getAnnotatedType().getJavaClass();
        watchForPermitParentFree(resource);
    }
    
    public void watchForPermitParentType(Class<?> resource) {
        Annotation[] anns = resource.getAnnotations();
        for (Object ann_ : anns) {
            if (ann_ instanceof PermitParentType) {
                PermitParentType ann = (PermitParentType)ann_;

                for (Class<?> parentType : ann.value()) {
                    allowedParents.put(resource, parentType);
                    allowedChildren.put(parentType, resource);
                    allowedParentsSimple.put(resource.getName(), parentType.getName());
                    allowedChildrenSimple.put(parentType.getName(), resource.getName());
                }
            }
        }
    }

    public void watchForPermitParentFree(Class<?> resource) {
        allowedParentFree.add(resource);
        allowedParentFreeSimple.add(resource.getName());
    }

    public Set<Class<?>> getAllowedParentFree() {
        return this.allowedParentFree;
    }
    
    public Multimap<Class<?>, Class<?>> getAllowedParents() {
        return this.allowedParents;
    }
    
    public Multimap<Class<?>, Class<?>> getAllowedChildren() {
        return this.allowedChildren;
    }

    public Set<String> getAllowedParentFreeSimple() {
        return this.allowedParentFreeSimple;
    }

    public Multimap<String, String> getAllowedParentsSimple() {
        return this.allowedParentsSimple;
    }
    
    public Multimap<String, String> getAllowedChildrenSimple() {
        return this.allowedChildrenSimple;
    }
}
