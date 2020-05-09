package com.tokera.ate.io.api;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.msg.MessageDataMetaDto;

import java.util.List;

public interface IHookContext extends IHookFeed {

    IPartitionKey partitionKey();

    <T extends BaseDao> void addHook(IHookCallback<T> callback, Class<T> clazz);

    <T extends BaseDao> boolean removeHook(IHookCallback<T> callback, Class<T> clazz);

    boolean isEmpty();

    void clean();
}
