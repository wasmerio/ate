package com.tokera.ate.delegates;

import com.tokera.ate.annotations.*;
import com.tokera.ate.dao.enumerations.RiskRole;
import com.tokera.ate.dao.enumerations.UserRole;
import com.tokera.ate.scopes.ResourceScoped;

import javax.ws.rs.WebApplicationException;
import javax.ws.rs.container.ResourceInfo;
import javax.ws.rs.core.Response;
import java.lang.reflect.Method;
import java.util.*;
import java.util.stream.Collectors;

/**
 * Delegate used to cache the retrieve details about the currentRights executing REST method (e.g. any permission restrictions
 * that it may have).
 */
@ResourceScoped
public class ResourceInfoDelegate
{
    private AteDelegate d = AteDelegate.getUnsafe();

    private final Method resourceMethod;
    private final Class<?> resourceClazz;
    private final boolean permitMissingToken;
    private final Iterable<RiskRole> permitRiskRoles;
    private final Iterable<UserRole> permitUserRoles;
    private final Iterable<PermitReadEntity> permitReadParams;
    private final Iterable<PermitWriteEntity> permitWriteParams;

    public ResourceInfoDelegate() {
        ResourceInfo resourceInfo = d.resourceScopeInterceptor.getResourceInfoOrNull();
        if (resourceInfo == null) {
            throw new WebApplicationException("Access denied (missing currentRights method)",
                    Response.Status.UNAUTHORIZED);
        }
        Method method = resourceInfo.getResourceMethod();
        this.resourceMethod = method;
        this.resourceClazz = method.getDeclaringClass();
        this.permitMissingToken = ResourceInfoDelegate.computeAllowMissingToken(this.resourceClazz, method);
        this.permitRiskRoles = ResourceInfoDelegate.computeRiskRoles(this.resourceClazz, method);
        this.permitUserRoles = ResourceInfoDelegate.computeUserRoles(this.resourceClazz, method);
        this.permitReadParams = ResourceInfoDelegate.computePermitReadParam(this.resourceClazz, method);
        this.permitWriteParams = ResourceInfoDelegate.computePermitWriteParam(this.resourceClazz, method);
    }

    private static boolean computeAllowMissingToken(Class<?> clazz, Method method) {
        PermitMissingToken permitMissingTokenAtt = method.getAnnotation(PermitMissingToken.class);
        for (Class<?> search = clazz; permitMissingTokenAtt == null && search != null; search = search.getSuperclass())
            permitMissingTokenAtt = search.getAnnotation(PermitMissingToken.class);
        return permitMissingTokenAtt != null;
    }

    private static Iterable<RiskRole> computeRiskRoles(Class<?> clazz, Method method) {
        PermitRiskRole permitRiskRoleAtt = method.getAnnotation(PermitRiskRole.class);
        for (Class<?> search = clazz; permitRiskRoleAtt == null && search != null; search = search.getSuperclass())
            permitRiskRoleAtt = search.getAnnotation(PermitRiskRole.class);

        if (permitRiskRoleAtt != null) {
            return Arrays.stream(permitRiskRoleAtt.value()).collect(Collectors.toList());
        } else {
            return new ArrayList<>();
        }
    }

    private static Iterable<UserRole> computeUserRoles(Class<?> clazz, Method method) {
        PermitUserRole permitUserRoleAtt = method.getAnnotation(PermitUserRole.class);
        for (Class<?> search = clazz; permitUserRoleAtt == null && search != null; search = search.getSuperclass())
            permitUserRoleAtt = search.getAnnotation(PermitUserRole.class);

        if (permitUserRoleAtt != null) {
            return Arrays.stream(permitUserRoleAtt.value()).collect(Collectors.toList());
        } else {
            return new ArrayList<>();
        }
    }

    private static Iterable<PermitReadEntity> computePermitReadParam(Class<?> clazz, Method method) {
        PermitReadEntities permitReadsAtt = method.getAnnotation(PermitReadEntities.class);
        for (Class<?> search = clazz; permitReadsAtt == null && search != null; search = search.getSuperclass())
            permitReadsAtt = search.getAnnotation(PermitReadEntities.class);

        if (permitReadsAtt != null) {
            return Arrays.stream(permitReadsAtt.value()).collect(Collectors.toList());
        } else {
            return new ArrayList<>();
        }
    }

    private static Iterable<PermitWriteEntity> computePermitWriteParam(Class<?> clazz, Method method) {
        PermitWriteEntities permitWritesAtt = method.getAnnotation(PermitWriteEntities.class);
        for (Class<?> search = clazz; permitWritesAtt == null && search != null; search = search.getSuperclass())
            permitWritesAtt = search.getAnnotation(PermitWriteEntities.class);

        if (permitWritesAtt != null) {
            return Arrays.stream(permitWritesAtt.value()).collect(Collectors.toList());
        } else {
            return new ArrayList<>();
        }
    }

    /**
     * @return Class that was invoked by an external caller
     */
    public Class<?> getResourceClazz() {
        return resourceClazz;
    }

    /**
     * @return Method that was invoked by an external caller
     */
    public Method getResourceMethod() {
        return resourceMethod;
    }

    /**
     * @return True if this method can be invoked without a valid token
     */
    public boolean isPermitMissingToken() {
        return permitMissingToken;
    }

    /**
     * @return List of all the risk role types that are allowed to access this resource
     */
    public Iterable<RiskRole> getPermitRiskRoles() {
        return permitRiskRoles;
    }

    /**
     * @return List of all the user role types that are allowed to access this resource
     */
    public Iterable<UserRole> getPermitUserRoles() {
        return permitUserRoles;
    }

    /**
     * @return List of all the minimum read permissions the active token must own for this resource
     */

    public Iterable<PermitReadEntity> getPermitReadParams() {
        return permitReadParams;
    }

    /**
     * @return List of all the minimum write permissions the active token must own for this resource
     */
    public Iterable<PermitWriteEntity> getPermitWriteParams() {
        return permitWriteParams;
    }
}