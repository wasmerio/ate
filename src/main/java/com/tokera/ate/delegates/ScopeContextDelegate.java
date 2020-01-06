package com.tokera.ate.delegates;

import com.tokera.ate.scopes.IScopeContext;
import com.tokera.ate.scopes.Startup;

import javax.enterprise.context.ApplicationScoped;
import java.util.ArrayList;
import java.util.List;

@Startup
@ApplicationScoped
public class ScopeContextDelegate {
    private final AteDelegate d = AteDelegate.get();

    private static ArrayList<IScopeContext> scopeContexts = new ArrayList<>();

    public static void registerScopeContext(IScopeContext context) {
        scopeContexts.add(context);
    }

    public List<IScopeContext> getScopeContexts() {
        return this.scopeContexts;
    }
}
