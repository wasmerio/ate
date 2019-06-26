package com.tokera.examples.common;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.io.api.ITaskCallback;
import com.tokera.ate.scopes.Startup;
import com.tokera.examples.dao.CoinShare;

import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;

@Startup
@ApplicationScoped
public class CoinWatcherTask implements ITaskCallback<CoinShare> {
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;

    public void onCreate(CoinShare obj) {

    }

    @Override
    public void onInit(CoinShare coinShare) {
        LOG.info("coin-share:init:" + coinShare.id);
    }

    @Override
    public void onData(CoinShare coinShare) {
        AteDelegate d = AteDelegate.get();
        BaseDao parent = d.daoHelper.getParent(coinShare);
        LOG.info("coin-share:data:" + coinShare.id);
    }

    @Override
    public void onRemove(PUUID id) {
        LOG.info("coin-share:create:" + id);
    }
}
