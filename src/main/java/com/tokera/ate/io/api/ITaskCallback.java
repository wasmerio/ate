package com.tokera.ate.io.api;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;

import java.util.Collection;
import java.util.UUID;

/**
 * Interface that's invoked when a particular data object on a particular partition is created, modifeid or removed
 */
public interface ITaskCallback<T extends BaseDao> {

    /**
     * Unique ID of this callback
     * @return
     */
    UUID id();

    /**
     * Callback invoked for every object of this type that is discovered when the task processor first starts up, after
     * its in an operational state the onCreate callback will be called for all newly created objects
     */
    default void onInit(Collection<T> obj, ITask task) {
    }

    /**
     * Callback invoked when an object is encountered for the first time
     */
    default void onCreate(T obj, ITask task) {
    }

    /**
     * Callback invoked whenever a data object is created or updated
     */
    default void onUpdate(T obj, ITask task) {
    }

    /**
     * Callback invoked whenever a data object is removed
     */
    default void onRemove(PUUID id, ITask task) {
    }

    /**
     * Callback invoked every tick of time that passes (defaults to 10 seconds)
     */
    default void onTick(ITask task) {
    }

    /**
     * Callback invoked every tick of time that passes and its been idle (defaults to 10 seconds)
     */
    default void onIdle(ITask task) {
    }
}
