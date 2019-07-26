package com.tokera.ate.delegates;

import com.tokera.ate.scopes.Startup;

import javax.enterprise.context.ApplicationScoped;

@Startup
@ApplicationScoped
public class ProducerDelegate {
    AteDelegate d = AteDelegate.get();
}
