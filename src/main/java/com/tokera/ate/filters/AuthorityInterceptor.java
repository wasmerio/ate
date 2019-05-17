package com.tokera.ate.filters;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.events.TokenScopeChangedEvent;
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
    private int inferredTopic = 0;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private DefaultBootstrapInit interceptorInit;
    
    public static final String HEADER_AUTHORIZATION = "Authorization";
    public static boolean c_logVerbose = false;

    @PostConstruct
    public void init() {
        interceptorInit.touch();
    }

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
        String tokenHash = AuthorityInterceptor.getHeaderString(requestContext, HEADER_AUTHORIZATION);
        if (tokenHash != null) {
            if (AuthorityInterceptor.c_logVerbose == true) {
                this.LOG.info("found header(Authentication) cookie");
            }
        }

        // ...or from the query string
        if (tokenHash == null) {
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
                                tokenHash = pair.getValue()[0];
                            }
                        }
                    } catch (IllegalArgumentException e) {
                        throw new RuntimeException("Illegal argument while parsing query string [str=" + queryStr + "]", e);
                    }
                }
            }
        }

        // ... or maybe its in the query string of the original currentRights (in a redirect scenario)
        if (tokenHash == null) {
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
                            tokenHash = comps[1];
                        }
                    }
                }
            }
        }

        // ...or from a cookie
        if (tokenHash == null) {
            Cookie cookie = MapTools.getOrNull(requestContext.getCookies(), "token");
            if (cookie != null) {
                if (AuthorityInterceptor.c_logVerbose == true) {
                    this.LOG.info("found token cookie");
                }
                tokenHash = cookie.getValue();
            }
        }

        // If we are in a token scope
        if (tokenHash != null)
        {
            // Enter the token scope
            d.currentToken.enterTokenScope(tokenHash);
        }
        
        // Now perform all the security checks
        d.currentToken.validate();
    }

    private void undoInferredTopic() {
        if (inferredTopic > 0) {
            d.requestContext.popPartitionKey();
            inferredTopic--;
        }
    }

    public void foundToken(@Observes TokenScopeChangedEvent discovery)
    {
        // If we are already in an inferred topic then leave it
        undoInferredTopic();

        // If we dont have a topic set from the headers then we can
        // just use the one thats passed in the token
        d.requestContext.pushPartitionKey(discovery.getPartitionKey());
        inferredTopic++;
    }

    @Override
    public void filter(ContainerRequestContext requestContext, ContainerResponseContext responseContext)
    {
        undoInferredTopic();

        // If the token exists then set the response
        TokenDto token = d.currentToken.getTokenOrNull();
        if (token != null)
        {
            // Set the response headers
            javax.servlet.http.Cookie cookieToken = new javax.servlet.http.Cookie("token", token.getHash());
            cookieToken.setSecure(true);
            cookieToken.setHttpOnly(true);

            HttpServletResponse response = this.response;
            if (response != null) {
                response.addHeader("Authorization", token.getHash());
                response.addCookie(cookieToken);
            }
        }    
        
        d.currentToken.leaveTokenScope();
    }
}
