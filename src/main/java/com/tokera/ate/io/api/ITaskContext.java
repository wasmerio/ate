package com.tokera.ate.io.api;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.MessageDataMetaDto;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.List;

public interface ITaskContext
{
    IPartitionKey partitionKey();

    void feed(MessageDataMetaDto msg);

    <T extends BaseDao> ITask addTask(ITaskCallback<T> callback, Class<T> clazz, int idleTime, @Nullable TokenDto token);

    <T extends BaseDao> boolean removeTask(ITaskCallback<T> callback, Class<T> clazz);

    boolean isEmpty();

    void clean();

    void destroyAll();
}
