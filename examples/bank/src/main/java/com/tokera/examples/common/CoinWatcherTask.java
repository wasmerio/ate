package com.tokera.examples.common;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.io.api.ITask;
import com.tokera.ate.io.api.ITaskCallback;
import com.tokera.ate.scopes.Startup;
import com.tokera.examples.dao.CoinShare;

import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import java.util.Collection;
import java.util.UUID;

@Startup
@ApplicationScoped
public class CoinWatcherTask implements ITaskCallback<CoinShare> {
    private UUID id = UUID.randomUUID();

    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;

    @Override
    public UUID id() {
        return this.id;
    }

    @Override
    public void onUpdate(CoinShare coinShare, ITask task) {
        process(coinShare);
    }

    @Override
    public void onCreate(CoinShare coinShare, ITask task) {
        process(coinShare);
    }

    public void process(CoinShare coinShare) {
        AteDelegate d = AteDelegate.get();
        LOG.info("coin-share:data:" + coinShare.id);
    }
}
