package com.tokera.examples.rest;

import com.tokera.ate.annotations.PermitUserRole;
import com.tokera.ate.dao.enumerations.UserRole;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.examples.common.CoinPartitionKey;
import com.tokera.examples.common.CoinWatcherTask;
import com.tokera.examples.dao.CoinShare;

import javax.enterprise.context.RequestScoped;
import javax.inject.Inject;
import javax.ws.rs.DELETE;
import javax.ws.rs.GET;
import javax.ws.rs.Path;

@RequestScoped
@Path("/task")
public class TaskREST {
    AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private CoinWatcherTask coinWatcher;

    @GET
    @Path("coinWatcher")
    @PermitUserRole(UserRole.ANYTHING)
    public void startCoinWatcher() {
        d.taskManager.subscribe(new CoinPartitionKey(), CoinShare.class, coinWatcher);
    }

    @DELETE
    @Path("coinWatcher")
    @PermitUserRole(UserRole.ANYTHING)
    public void stopCoinWatcher() {
        d.taskManager.unsubscribeByCallback(coinWatcher);
    }
}
