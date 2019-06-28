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

@Startup
@ApplicationScoped
public class CoinWatcherTask implements ITaskCallback<CoinShare> {
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;

    @Override
    public void onInit(Collection<CoinShare> coinShares, ITask task) {
        for (CoinShare coinShare : coinShares) {
            LOG.info("coin-share:init:" + coinShare.id);
        }
    }

    @Override
    public void onData(CoinShare coinShare, ITask task) {
        AteDelegate d = AteDelegate.get();
        BaseDao parent = d.daoHelper.getParent(coinShare);
        LOG.info("coin-share:data:" + coinShare.id);
    }

    @Override
    public void onRemove(PUUID id, ITask task) {
        LOG.info("coin-share:create:" + id);
    }

    @Override
    public void onTick(ITask iTask) {
    }
}
