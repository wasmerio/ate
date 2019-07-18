package com.tokera.ate.io.api;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.msg.MessageDataMetaDto;

public interface IHookContext {

    IPartitionKey partitionKey();

    void feed(MessageDataMetaDto msg);

    <T extends BaseDao> IHookCallback<T> callback(Class<T> clazz);
}
