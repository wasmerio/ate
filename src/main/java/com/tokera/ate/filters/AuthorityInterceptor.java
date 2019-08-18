package com.tokera.ate.filters;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.events.TokenDiscoveryEvent;
import com.tokera.ate.events.TokenScopeChangedEvent;
import com.tokera.ate.io.api.IPartitionKey;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.annotation.PostConstruct;
import javax.enterprise.context.RequestScoped;
import javax.enterprise.event.Observes;
import javax.inject.Inject;
import javax.ws.rs.container.ContainerRequestContext;
import javax.ws.rs.container.ContainerRequestFilter;
import javax.ws.rs.container.ContainerResponseContext;
import javax.ws.rs.container.ContainerResponseFilter;
import javax.ws.rs.core.Context;
import javax.ws.rs.ext.Provider;
import java.util.Map;
import javax.annotation.Priority;
import javax.servlet.http.HttpServletRequest;
import javax.servlet.http.HttpServletResponse;
import javax.ws.rs.Priorities;
import javax.ws.rs.core.Cookie;

/**
 * Filter that will read and process tokens that are passed to the API REST calls
 */
@RequestScoped
@Provider
@Priority(Priorities.AUTHORIZATION)
public class AuthorityInterceptor implements ContainerRequestFilter, ContainerResponseFilter {

    protected AteDelegate d = AteDelegate.get();

    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;
    private @Context @Nullable HttpServletRequest request;
    private @Context @Nullable HttpServletResponse response;
    private int inferredPartition = 0;
    
    public static final String HEADER_AUTHORIZATION = "Authorization";
    public static boolean c_logVerbose = false;

    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible"})       // Fixed an issue where the return type is actually nullable
    private static @Nullable String getHeaderString(ContainerRequestContext requestContext, String header) {
        return requestContext.getHeaderString(header);
    }

    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible"})       // Fixed an issue where the return type is actually nullable
    private static @Nullable String getRequestQueryString(HttpServletRequest requestContext) {
        return requestContext.getQueryString();
    }

    @Override
    @SuppressWarnings( "deprecation" )
    public void filter(ContainerRequestContext requestContext)
    {
        // Set the requestContext variable
        d.requestContext.setContainerRequestContext(requestContext);

        // Extract the token (either from the authorization header)
        String token = AuthorityInterceptor.getHeaderString(requestContext, HEADER_AUTHORIZATION);
        if (token != null) {
            if (AuthorityInterceptor.c_logVerbose == true) {
                this.LOG.info("found header(Authentication) cookie");
            }
        }

        // ...or from the query string
        if (token == null) {
            HttpServletRequest request = this.request;
            if (request != null) {
                String queryStr = AuthorityInterceptor.getRequestQueryString(request);
                if (queryStr != null) {
                    this.LOG.info("queryString: " + queryStr);
                    try {
                        for (Map.Entry<String, String[]> pair : javax.servlet.http.HttpUtils.parseQueryString(queryStr).entrySet()) {
                            if (pair.getValue().length <= 0) continue;
                            if (pair.getKey().equalsIgnoreCase("token")) {
                                this.LOG.info("found querystring cookie");
                                token = pair.getValue()[0];
                            }
                        }
                    } catch (IllegalArgumentException e) {
                        throw new RuntimeException("Illegal argument while parsing query string [str=" + queryStr + "]", e);
                    }
                }
            }
        }

        // ... or maybe its in the query string of the original currentRights (in a redirect scenario)
        if (token == null) {
            String origUri = AuthorityInterceptor.getHeaderString(requestContext, "X-Original-URI");
            if (origUri != null) {
                if (origUri.contains("?")) {
                    origUri = origUri.substring(origUri.indexOf("?") + 1);
                }
                if (origUri.contains("&")) {
                    for (String pair : origUri.split("&")) {
                        String[] comps = pair.split("=");
                        if (comps.length < 2) continue;
                        if (comps[0].equalsIgnoreCase("token"))
                        {
                            if (AuthorityInterceptor.c_logVerbose == true) {
                                this.LOG.info("found header(X-Original-URI) cookie");
                            }
                            token = comps[1];
                        }
                    }
                }
            }
        }

        // ...or from a cookie
        if (token == null) {
            Cookie cookie = MapTools.getOrNull(requestContext.getCookies(), "token");
            if (cookie != null) {
                if (AuthorityInterceptor.c_logVerbose == true) {
                    this.LOG.info("found token cookie");
                }
                token = cookie.getValue();
            }
        }

        // If we are in a token scope
        if (token != null)
        {
            // Enter the token scope
            d.currentToken.enterTokenScope(token);
        }
        
        // Now perform all the security checks
        d.currentToken.validate();

        // Finally complete any sync operations
        if (token != null) {
            d.transaction.finish();
        }
    }

    private void undoInferredPartition() {
        if (inferredPartition > 0) {
            d.requestContext.popPartitionKey();
            inferredPartition--;
        }
    }

    public void foundToken(@Observes TokenScopeChangedEvent discovery)
    {
        // If we are already in an inferred partition then leave it
        undoInferredPartition();

        // If we dont have a partition set from the headers then we can
        // just use the one thats passed in the token
        IPartitionKey partitionKey = discovery.getPartitionKey();
        if (partitionKey != null) {
            d.requestContext.pushPartitionKey(partitionKey);
            inferredPartition++;
        }
    }

    @Override
    public void filter(ContainerRequestContext requestContext, ContainerResponseContext responseContext)
    {
        undoInferredPartition();

        // If the token exists then set the response
        TokenDto token = d.currentToken.getTokenOrNull();
        if (token != null)
        {
            // Set the response headers
            javax.servlet.http.Cookie cookieToken = new javax.servlet.http.Cookie("token", token.getBase64());
            cookieToken.setSecure(true);
            cookieToken.setHttpOnly(true);

            HttpServletResponse response = this.response;
            if (response != null) {
                response.addHeader("Authorization", token.getBase64());
                response.addCookie(cookieToken);
            }
        }    
        
        d.currentToken.leaveTokenScope();
    }
}
