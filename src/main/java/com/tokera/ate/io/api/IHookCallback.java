package com.tokera.ate.io.api;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;

import java.util.Collection;

/**
 * Interface that's invoked when a particular data object on a particular partition is created, modifeid or removed
 */
public interface IHookCallback<T extends BaseDao> {

    /**
     * Callback invoked whenever a data object is created or updated
     */
    void onData(T obj, IHookContext context);

    /**
     * Callback invoked whenever a data object is removed
     */
    void onRemove(PUUID id, IHookContext context);
}
