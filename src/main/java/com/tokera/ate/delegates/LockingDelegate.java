package com.tokera.ate.delegates;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.scopes.Startup;

import javax.enterprise.context.ApplicationScoped;
import java.util.concurrent.ConcurrentHashMap;

@Startup
@ApplicationScoped
public class LockingDelegate {

    private final ConcurrentHashMap<PUUID, Object> lockingContext;

    public LockingDelegate() {
        this.lockingContext = new ConcurrentHashMap<>();
    }

    public Object lockable(PUUID pid)
    {
        return lockingContext.computeIfAbsent(pid, a -> new Object());
    }
}
