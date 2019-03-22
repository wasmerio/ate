/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.scope;

import com.tokera.server.api.security.TokenSecurity;
import org.tomitribe.microscoped.core.ScopeContext;

import javax.enterprise.event.Observes;
import javax.enterprise.inject.spi.AfterBeanDiscovery;
import javax.enterprise.inject.spi.BeforeBeanDiscovery;
import javax.enterprise.inject.spi.Extension;

/**
 *
 * @author John
 */
public class TokenScopeExtension implements Extension {
    
    private static final ScopeContext<String> context = new ScopeContext<>(TokenScoped.class, TokenSecurity.SECURITY_TOKEN_CACHE);

    public static ScopeContext<String> getContext() {
        return context;
    }

    public void beforeBeanDiscovery(@Observes BeforeBeanDiscovery bbd) {
        
        bbd.addScope(TokenScoped.class, true, false);
    }

    public void afterBeanDiscovery(@Observes AfterBeanDiscovery abd) {
        
        abd.addContext(getContext());
    }
}
