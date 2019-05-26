package com.tokera.ate.extensions;

import com.google.common.collect.HashMultimap;
import com.google.common.collect.Multimap;
import com.tokera.ate.annotations.*;
import com.tokera.ate.dao.IRoles;
import com.tokera.ate.dao.base.BaseDao;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.lang.annotation.Annotation;
import java.lang.reflect.Field;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import javax.enterprise.context.Dependent;
import javax.enterprise.event.Observes;
import javax.enterprise.inject.spi.Extension;
import javax.enterprise.inject.spi.ProcessAnnotatedType;
import javax.enterprise.inject.spi.WithAnnotations;

/**
 * Extension that will build a graph of allowed parent/child relationships for all data objects
 */
public class DaoParentDiscoveryExtension implements Extension {

    private final Map<Class<?>, String> allowedImplicitAuthority = new HashMap<>();
    private final Map<Class<?>, Field> allowedDynamicImplicitAuthority = new HashMap<>();
    private final HashSet<Class<?>> allowedParentClaimable = new HashSet<>();
    private final HashSet<Class<?>> allowedParentFree = new HashSet<>();
    private final Multimap<Class<?>, Class<?>> allowedParents = HashMultimap.create();
    private final Multimap<Class<?>, Class<?>> allowedChildren = HashMultimap.create();
    private final Map<String, String> allowedImplicitAuthoritySimple = new HashMap<>();
    private final Map<String, Field> allowedDynamicImplicitAuthoritySimple = new HashMap<>();
    private final HashSet<String> allowedParentFreeSimple = new HashSet<>();
    private final HashSet<String> allowedParentClaimableSimple = new HashSet<>();
    private final Multimap<String, String> allowedParentsSimple = HashMultimap.create();
    private final Multimap<String, String> allowedChildrenSimple = HashMultimap.create();

    public DaoParentDiscoveryExtension() {
    }
    
    public void watchForPermitParentType(@Observes @WithAnnotations(PermitParentType.class) ProcessAnnotatedType processAnnotatedType) {
        Class<?> resource = processAnnotatedType.getAnnotatedType().getJavaClass();
        validateDaoObject(resource, false);
        watchForPermitParentType(resource);
        watchForImplicitAuthority(resource);
    }

    public void watchForPermitParentFree(@Observes @WithAnnotations(PermitParentFree.class) ProcessAnnotatedType processAnnotatedType) {
        Class<?> resource = processAnnotatedType.getAnnotatedType().getJavaClass();
        validateDaoObject(resource, true);
        watchForPermitParentFree(resource);
        watchForImplicitAuthority(resource);
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

        if (resource.getAnnotation(ClaimableAuthority.class) != null) {
            allowedParentClaimable.add(resource);
            allowedParentClaimableSimple.add(resource.getName());
        }
    }

    private @Nullable Field findDynamicImplicitAuthority(Class<?> resource) {
        for (Field field : resource.getFields()) {
            if (field.getAnnotation(ImplicitAuthorityField.class) != null) {
                return field;
            }
        }
        Class<?> parent = resource.getSuperclass();
        if (parent == null) return null;
        if (parent == Object.class) return null;
        return findDynamicImplicitAuthority(parent);
    }

    public void watchForImplicitAuthority(Class<?> resource) {
        ImplicitAuthority implicitAuthority = resource.getAnnotation(ImplicitAuthority.class);
        if (implicitAuthority != null) {
            allowedImplicitAuthority.put(resource, implicitAuthority.value());
            allowedImplicitAuthoritySimple.put(resource.getName(), implicitAuthority.value());
        }

        Field implicitAuthorityField = findDynamicImplicitAuthority(resource);
        if (implicitAuthorityField != null) {
            allowedDynamicImplicitAuthority.put(resource, implicitAuthorityField);
            allowedDynamicImplicitAuthoritySimple.put(resource.getName(), implicitAuthorityField);
        }
    }

    public Map<Class<?>, String> getAllowedImplicitAuthority() {
        return this.allowedImplicitAuthority;
    }

    public Map<Class<?>, Field> getAllowedDynamicImplicitAuthority() {
        return this.allowedDynamicImplicitAuthority;
    }

    public Set<Class<?>> getAllowedParentFree() {
        return this.allowedParentFree;
    }

    public Set<Class<?>> getAllowedParentClaimable() {
        return this.allowedParentFree;
    }
    
    public Multimap<Class<?>, Class<?>> getAllowedParents() {
        return this.allowedParents;
    }
    
    public Multimap<Class<?>, Class<?>> getAllowedChildren() {
        return this.allowedChildren;
    }

    public Map<String, String> getAllowedImplicitAuthoritySimple() {
        return this.allowedImplicitAuthoritySimple;
    }

    public Map<String, Field> getAllowedDynamicImplicitAuthoritySimple() {
        return this.allowedDynamicImplicitAuthoritySimple;
    }

    public Set<String> getAllowedParentFreeSimple() {
        return this.allowedParentFreeSimple;
    }

    public Set<String> getAllowedParentClaimableSimple() {
        return this.allowedParentFreeSimple;
    }

    public Multimap<String, String> getAllowedParentsSimple() {
        return this.allowedParentsSimple;
    }
    
    public Multimap<String, String> getAllowedChildrenSimple() {
        return this.allowedChildrenSimple;
    }

    private int countDynamicImplicitAuthority(Class<?> resource) {
        int ret = 0;
        for (Field field : resource.getFields()) {
            if (field.getAnnotation(ImplicitAuthorityField.class) != null) {
                ret++;
            }
        }
        Class<?> parent = resource.getSuperclass();
        if (parent == null) return ret;
        if (parent == Object.class) return ret;
        return ret + countDynamicImplicitAuthority(parent);
    }

    private void validateDaoObject(Class<?> clazz, boolean mustHaveRights)
    {
        boolean implicitAuthority = clazz.isAnnotationPresent(ImplicitAuthority.class) || findDynamicImplicitAuthority(clazz) != null;
        if (countDynamicImplicitAuthority(clazz) > 1) {
            throw new RuntimeException("The type [" + clazz.getSimpleName() + "] is marked with ImplicitAuthorityField more than once which is not allowed.");
        }

        if (clazz.isAnnotationPresent(PermitParentType.class) && implicitAuthority) {
            throw new RuntimeException("The type [" + clazz.getSimpleName() + "] is marked with PermitParentType and ImplicitAuthority which is not allowed as it breaks the chains-of-trust.");
        }

        if (clazz.isAnnotationPresent(PermitParentFree.class) == false && implicitAuthority) {
            throw new RuntimeException("The type [" + clazz.getSimpleName() + "] is marked with ImplicitAuthority but not PermitParentFree which is required as implicit authority can only be granted to the root of the chains-of-trust.");
        }

        if (clazz.isAnnotationPresent(Dependent.class) == false) {
            throw new RuntimeException("The type [" + clazz.getSimpleName() + "] is marked with PermitParentType or PermitParentFree annotations thus it must also be a bean marked with the Dependent annotation.");
        }

        if (BaseDao.class.isAssignableFrom(clazz) == false) {
            throw new RuntimeException("The type [" + clazz.getSimpleName() + "] is marked with PermitParentType or PermitParentFree annotations thus it is a data access object and hence needs to inherit from " + BaseDao.class.getSimpleName() + ".");
        }

        if (mustHaveRights) {
            if (IRoles.class.isAssignableFrom(clazz) == false) {
                throw new RuntimeException("The type [" + clazz.getSimpleName() + "] is marked with PermitParentFree annotation and thus it is at the top of the trust chain hence it must inherit from " + IRoles.class.getSimpleName() + ".");
            }
        }
    }
}
