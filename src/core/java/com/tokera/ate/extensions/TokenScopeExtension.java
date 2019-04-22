/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.extensions;

import com.tokera.ate.scopes.TokenScoped;

import javax.enterprise.event.Observes;
import javax.enterprise.inject.spi.AfterBeanDiscovery;
import javax.enterprise.inject.spi.BeforeBeanDiscovery;
import javax.enterprise.inject.spi.Extension;
import org.tomitribe.microscoped.core.ScopeContext;

/**
 * Extension that injects the token scope into the dependency injection system
 */
public class TokenScopeExtension implements Extension {

    // number of minutes that token scopes should be allowed to be idle for before cleanup
    public static final long SECURITY_TOKEN_CACHE = 3L;

    public void beforeBeanDiscovery(@Observes BeforeBeanDiscovery bbd) {
        bbd.addScope(TokenScoped.class, true, false);
    }

    public void afterBeanDiscovery(@Observes AfterBeanDiscovery abd) {
        abd.addContext(new ScopeContext<>(TokenScoped.class, TokenScopeExtension.SECURITY_TOKEN_CACHE));
    }
}
