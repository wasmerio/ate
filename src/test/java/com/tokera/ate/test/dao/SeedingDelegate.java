package com.tokera.ate.test.dao;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.scopes.Startup;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;

import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;

@Startup
@ApplicationScoped
public class SeedingDelegate {
    private AteDelegate d = AteDelegate.get();
    private @MonotonicNonNull PrivateKeyWithSeedDto rootkey;

    public PrivateKeyWithSeedDto getRootKey() {
        assert rootkey != null : "@AssumeAssertion(nullness): Must not be null";
        return rootkey;
    }

    @PostConstruct
    public void init() {
        rootkey = d.encryptor.genSignKeyAndSeed();
    }
}
