package com.tokera.ate.io.api;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.msg.MessageDataMetaDto;

import java.util.List;

public interface ITaskContext
{
    IPartitionKey partitionKey();

    void feed(MessageDataMetaDto msg);

    <T extends BaseDao> ITask addTask(ITaskCallback<T> callback, Class<T> clazz);

    boolean removeTask(ITask task);

    List<ITask> tasks();
}
