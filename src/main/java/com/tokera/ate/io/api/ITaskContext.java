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

    boolean removeTask(ITask task);

    <T extends BaseDao> void addHook(IHookCallback<T> callback, Class<T> clazz);

    <T extends BaseDao> boolean removeHook(IHookCallback<T> callback, Class<T> clazz);

    List<ITask> tasks();
}
