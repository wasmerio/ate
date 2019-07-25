package com.tokera.ate.io.api;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;

import java.util.Collection;
import java.util.UUID;

/**
 * Interface that's invoked when a particular data object on a particular partition is created, modifeid or removed
 */
public interface IHookCallback<T extends BaseDao> {

    /**
     * Unique ID of this callback
     * @return
     */
    UUID id();

    /**
     * Callback invoked whenever a data object is created or updated
     */
    default void onData(T obj, IHook context) {
    }

    /**
     * Callback invoked whenever a data object is removed
     */
    default void onRemove(PUUID id, IHook context) {
    }
}
