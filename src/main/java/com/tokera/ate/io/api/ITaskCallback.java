package com.tokera.ate.io.api;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;

/**
 * Interface that's invoked when a particular data object on a particular partition is created, modifeid or removed
 */
public interface ITaskCallback<T extends BaseDao> {

    /**
     * Callback invoked for every object of this type that is discovered when the task processor first starts up, after
     * its in an operational state the onCreate callback will be called for all newly created objects
     */
    void onInit(T obj, ITask task);

    /**
     * Callback invoked whenever a data object is created or updated
     */
    void onData(T obj, ITask task);

    /**
     * Callback invoked whenever a data object is removed
     */
    void onRemove(PUUID id, ITask task);

    /**
     * Callback invoked every tick of time that passes (defaults to 10 seconds)
     */
    void onTick(ITask task);
}
