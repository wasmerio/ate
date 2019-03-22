package com.tokera.ate.dao.io;

import com.tokera.server.api.delegate.MegaDelegate;
import com.tokera.server.api.qualifiers.CachingSystem;
import com.tokera.server.api.qualifiers.StorageSystem;
import com.tokera.server.api.repositories.DataRepository;

import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.inject.Instance;
import javax.enterprise.inject.Produces;
import javax.inject.Inject;

@ApplicationScoped
public class StorageFactory {

    private boolean testMode = false;

    private MegaDelegate d = MegaDelegate.getUnsafe();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private Instance<DataRepository> instRepo;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    @CachingSystem
    private Instance<CacheIO> instCache;

    @Produces
    @StorageSystem
    public ICloudIO createStorage()
    {
        return instRepo.get();
    }

    public boolean getTestMode() {
        return testMode;
    }

    public void setTestMode(boolean testMode) {
        this.testMode = testMode;
        d.dataSubscriber.setTestMode(testMode);
    }
}
