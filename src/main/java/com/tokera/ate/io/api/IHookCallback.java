package com.tokera.ate.io.api;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.msg.MessageDataMetaDto;

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
    default void onData(MessageDataMetaDto msg, IHook context) {
    }
}