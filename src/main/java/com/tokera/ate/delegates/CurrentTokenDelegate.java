package com.tokera.ate.delegates;

import com.tokera.ate.annotations.PermitReadEntity;
import com.tokera.ate.annotations.PermitWriteEntity;
import com.tokera.ate.common.UUIDTools;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.enumerations.PermissionPhase;
import com.tokera.ate.dao.enumerations.RiskRole;
import com.tokera.ate.dao.enumerations.UserRole;
import com.tokera.ate.dto.EffectivePermissions;
import com.tokera.ate.events.*;
import com.tokera.ate.scopes.TokenScoped;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.dto.TokenDto;
import org.checkerframework.checker.nullness.qual.Nullable;
import com.tokera.ate.scopes.ScopeContext;

import javax.enterprise.context.RequestScoped;
import javax.enterprise.event.Observes;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.container.ContainerRequestContext;
import javax.ws.rs.core.Response;
import java.util.UUID;

/**
 * Delegate used to interact with the currentRights authentication and authorization token that in the scope of this currentRights
 */
@RequestScoped
public class CurrentTokenDelegate {

    private AteDelegate                         d = AteDelegate.get();
    private boolean                             performedValidation = false;
    private boolean                             skipValidation = false;
    private boolean                             withinTokenScope = false;
    private @Nullable ScopeContext<String>      tokenScopeContext = null;
    private @Nullable String                    tokenScopeValue = null;
    private @Nullable String                    tokenScopeContextKey = null;

    public CurrentTokenDelegate() {
    }

    /**
     * Enters the Token scope hased on a hash of the token itself
     */
    @SuppressWarnings({"unchecked"})
    public void enterTokenScope(String token64)
    {
        d.requestAccessLog.pause();
        try {
            // Create the requestContext object
            ScopeContext<String> context = (ScopeContext<String>) d.beanManager.getContext(TokenScoped.class);
            this.tokenScopeValue = token64;
            this.tokenScopeContext = context;
            this.tokenScopeContextKey = context.enter(token64);

            boolean finished = false;
            this.withinTokenScope = true;
            try {
                TokenDto token = d.tokenSecurity.getToken();
                if (token != null) {
                    d.eventTokenScopeChanged.fire(new TokenScopeChangedEvent(token));
                }

                // Trigger the token scope entered flag
                d.eventTokenChanged.fire(new TokenStateChangedEvent());
                d.eventNewAccessRights.fire(new NewAccessRightsEvent());
                d.eventRightsValidation.fire(new RightsValidationEvent());
                finished = true;
            }
            finally
            {
                if (finished == false) {
                    this.withinTokenScope = false;
                }
            }
        } finally {
            d.requestAccessLog.unpause();
        }
    }

    /**
     * Leaves the token scope thus anything that requires a token will fail
     */
    public void leaveTokenScope()
    {
        // If we are within a token scope
        if (this.withinTokenScope == true)
        {
            // Clear everything
            this.withinTokenScope = false;

            try {
                // Grab the requestContext and leave it
                ScopeContext<String> context = this.tokenScopeContext;
                String contextKey = this.tokenScopeContextKey;
                if (context != null && contextKey != null) {
                    context.exit(contextKey);
                }

                // Clear the values
                this.tokenScopeContext = null;
                this.tokenScopeContextKey = null;
                this.tokenScopeValue = null;

            } catch (Throwable ex) {
                if (ex instanceof RuntimeException) {
                    throw (RuntimeException) ex;
                } else {
                    throw new WebApplicationException("Failed to end TokenContext - internal error", ex, Response.Status.INTERNAL_SERVER_ERROR);
                }
            }
        }
    }

    /**
     * Event that is triggered whenever the token state has changed (fired after the TokenDiscoveryEvent)
     */
    public void tokenChanged(@Observes RightsValidationEvent event)
    {
        validate();
    }

    /**
     * Gets the current token key
     */
    public String getTokenScopeValue() {
        return this.tokenScopeValue;
    }
    
    /**
     * Event that is triggered whenever a new Token is discovered (this event is fired before the TokenChanged event)
     */
    public void foundToken(@Observes TokenDiscoveryEvent discovery)
    {
        // We only need to fire the event if the token has actually changed
        TokenDto token = discovery.getToken();
        String oldToken = this.tokenScopeValue;
        if (oldToken != null && token.getBase64().equals(oldToken)) {
            return;
        }

        // Set the token
        if (skipValidation) {
            token.setValidated(true);
        }

        // Enter the token scope
        this.enterTokenScope(token.getBase64());
    }

    /**
     * @return True if the currentRights currentRights is within a Token scope and thus its safe to use operations that rely on
     * a Token and its metadata being present
     */
    public boolean getWithinTokenScope() {
        return withinTokenScope;
    }

    /**
     * @return Gets the currentRights token or throws an exception if none exists and/or we are not within a TokenScope
     */
    public TokenDto getToken()
    {
        TokenDto ret = getTokenOrNull();
        if (ret == null) {
            throw new WebApplicationException("Token is null.", Response.Status.BAD_REQUEST);
        }
        return ret;
    }

    /**
     * @return Gets the currentRights token or returns null if no token exists and/or we are not within a TokenScope
     */
    public @Nullable TokenDto getTokenOrNull()
    {
        if (this.withinTokenScope == false) {
            return null;
        }
        return d.tokenSecurity.getToken();
    }

    public void missingToken() {
        ContainerRequestContext request = this.d.requestContext.getContainerRequestContextOrNull();

        if (d.resourceScopeInterceptor.isActive() == false || d.resourceInfo.isPermitMissingToken() == false) {
            if (this.withinTokenScope)
            {
                // If we are in a token scope but dont have a token
                throw new WebApplicationException("Token is not known to this server - POST token to /login/token", Response.Status.PRECONDITION_FAILED);
            } else if (request != null) {
                throw new WebApplicationException("This operation requires a token (uri = '" + d.requestContext.getUriInfo().getAbsolutePath() + "')", Response.Status.UNAUTHORIZED);
            } else {
                throw new WebApplicationException("This operation requires a token.", Response.Status.UNAUTHORIZED);
            }
        }
    }

    /**
     * Validates that the currentRights execution requestContext is allowed for the currentRights Token
     */
    public void validate() {
        if (performedValidation == true) return;

        TokenDto token = this.getTokenOrNull();
        if (this.withinTokenScope == true && token == null) {
            this.missingToken();
        }

        validateRiskRole(token);
        validateUserRole(token);
        validateReadRoles(token);
        validateWriteRoles(token);

        performedValidation = true;
    }

    private void validateRiskRole(@Nullable TokenDto token)
    {
        // Make sure we have the risk role
        if (d.resourceScopeInterceptor.isActive() == false) return;
        for (RiskRole role : d.resourceInfo.getPermitRiskRoles()) {
            if (token == null) {
                missingToken();
            } else if (token.hasRiskRole(role) == false) {
                throw new WebApplicationException("Access denied (missing risk role)" + d.requestContext.getUriInfo().getAbsolutePath() + "')", Response.Status.UNAUTHORIZED);
            }
        }
    }

    private void validateUserRole(@Nullable TokenDto token)
    {
        if (d.resourceScopeInterceptor.isActive() == false) return;

        // Check if we allow any user roles for this operation
        for (UserRole role : d.resourceInfo.getPermitUserRoles()) {
            if (UserRole.ANYTHING.equals(role)) {
                if (token == null) {
                    missingToken();
                }
                return;
            }
        }

        // Make sure we have the user role
        boolean needsUserRole = false;
        boolean hasUserRole = false;
        for (UserRole role : d.resourceInfo.getPermitUserRoles()) {
            if (token == null) {
                missingToken();
            } else {
                needsUserRole = true;
                if (token.hasUserRole(role)) {
                    hasUserRole = true;
                }
            }
        }
        if (needsUserRole && !hasUserRole) {
            // We need a role that doesnt exist in the token
            StringBuilder rolesStr = new StringBuilder();
            for (UserRole role2 : d.resourceInfo.getPermitUserRoles()) {
                if (rolesStr.length() > 0) {
                    rolesStr.append(",");
                }
                rolesStr.append(role2);
            }
            throw new WebApplicationException("Access denied (missing user role: " + rolesStr.toString() + ") while processing " + d.requestContext.getUriInfo().getAbsolutePath() + "')",
                    Response.Status.UNAUTHORIZED);
        }

    }

    private void validateReadRoles(@Nullable TokenDto token)
    {
        if (d.resourceScopeInterceptor.isActive() == false) return;

        // Check all the read permissions
        for (PermitReadEntity paramRead : d.resourceInfo.getPermitReadParams()) {
            if (token == null) {
                missingToken();
            } else {
                for (String name : paramRead.name()) {
                    @DaoId PUUID entityId = this.getAndValidateRequestParamValue(name, paramRead.prefix());
                    boolean perm = d.authorization.canRead(entityId);
                    if (perm == false) {
                        RuntimeException ex;
                        try {
                            EffectivePermissions permissions = d.authorization.perms(null, entityId, PermissionPhase.BeforeMerge);
                            ex = d.authorization.buildReadException(permissions, true);
                        } catch (Throwable dump) {
                            ex = new WebApplicationException("Read access denied (Missing permitted entity). Path Param (" + name + "=" + entityId + ")",
                                    Response.Status.UNAUTHORIZED);
                        }
                        throw ex;
                    }
                }
            }
        }
    }

    private void validateWriteRoles(@Nullable TokenDto token)
    {
        if (d.resourceScopeInterceptor.isActive() == false) return;

        for (PermitWriteEntity paramWrite : d.resourceInfo.getPermitWriteParams()) {
            if (token == null) {
                missingToken();
            } else {
                for (String name : paramWrite.name()) {
                    @DaoId PUUID entityId = this.getAndValidateRequestParamValue(name, paramWrite.prefix());
                    boolean perm = d.authorization.canWrite(entityId);
                    if (perm == false) {
                        RuntimeException ex;
                        try {
                            EffectivePermissions permissions = d.authorization.perms(null, entityId, PermissionPhase.BeforeMerge);
                            ex = d.authorization.buildWriteException(permissions.rolesWrite, permissions, true);
                        } catch (Throwable dump) {
                            ex = new WebApplicationException("Write access denied (Missing permitted entity). Path Param (" + name + "=" + entityId + ")",
                                    Response.Status.UNAUTHORIZED);
                        }
                        throw ex;
                    }
                }
            }
        }
    }

    private @DaoId PUUID getAndValidateRequestParamValue(String name, @Nullable String _prefix) {
        if (d.requestContext.getUriInfo().getPathParameters().keySet().contains(name) == false) {
            throw new WebApplicationException("Access denied (Missing path parameter). Path Param Name:"
                    + name, Response.Status.UNAUTHORIZED);
        }
        String paramVal = d.requestContext.getUriInfo().getPathParameters().getFirst(name);

        String prefix = _prefix;

        PUUID pid;
        if (prefix != null && prefix.length() > 0) {
            UUID entityId = UUIDTools.generateUUID(prefix + paramVal);
            pid = PUUID.from(d.requestContext.currentPartitionKey(), entityId);
        } else {
            UUID entityId = UUIDTools.parseUUIDorNull(paramVal);
            if (entityId == null) {
                pid = PUUID.parse(paramVal);
            } else {
                pid = PUUID.from(d.requestContext.currentPartitionKey(), entityId);
            }
        }

        if (d.io.exists(pid) == false) {
            throw new WebApplicationException("Entity does not exist (" + name + "=" + paramVal + ", pid=" + pid.toString() + ")", Response.Status.NOT_FOUND);

        }
        
        return pid;
    }

    /**
     * Forces the state of the validation of the Token (normally used for debug and testing code)
     * @param flag
     */
    public void setPerformedValidation(boolean flag) {
        this.performedValidation = flag;
    }

    /**
     * @return True if the currentRights token exists and it contains a particular risk role
     */
    public boolean hasRiskRole(RiskRole role) {
        TokenDto token = this.getTokenOrNull();
        if (token == null) return false;
        return token.hasRiskRole(role);
    }

    /**
     * @return True if the currentRights token exists and it contains a particular user role
     */
    public boolean hasUserRole(UserRole role) {
        TokenDto token = this.getTokenOrNull();
        if (token == null) return false;
        return token.hasUserRole(role);
    }

    /**
     * Publishes a token that has been found or created in some way
     * @param token ReferenceNumber to the token to be published
     */
    public void publishToken(TokenDto token)
    {
        d.requestAccessLog.pause();
        try {
            // Fire an event that notifies about the discovery of a Token
            // (this event should be used by any beans in the TokenScope)
            TokenDiscoveryEvent discovery = new TokenDiscoveryEvent(token);
            d.eventTokenDiscovery.fire(discovery);

            // Trigger the token scope entered flag
            d.eventTokenScopeChanged.fire(new TokenScopeChangedEvent(token));
            d.eventTokenChanged.fire(new TokenStateChangedEvent());
            d.eventNewAccessRights.fire(new NewAccessRightsEvent());
            d.eventRightsValidation.fire(new RightsValidationEvent());
        } finally {
            d.requestAccessLog.unpause();
        }
    }

    public boolean isSkipValidation() {
        return skipValidation;
    }

    public void setSkipValidation(boolean skipValidation) {
        this.skipValidation = skipValidation;
    }
}