package com.tokera.ate.io.task;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.MessageDataMetaDto;
import com.tokera.ate.io.api.*;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.LinkedList;
import java.util.List;
import java.util.stream.Collectors;

/**
 * Represents a partition and class context that callbacks will be invoked under
 */
public class TaskContext<T extends BaseDao> implements ITaskContext {
    AteDelegate d = AteDelegate.get();

    public final IPartitionKey partitionKey;
    public final Class<T> clazz;
    public final List<TaskHandler<T>> taskHandlers;

    public TaskContext(IPartitionKey partitionKey, Class<T> clazz) {
        this.partitionKey = partitionKey;
        this.clazz = clazz;
        this.taskHandlers = new LinkedList<>();
    }

    @Override
    public IPartitionKey partitionKey() {
        return this.partitionKey;
    }

    @Override
    @SuppressWarnings("unchecked")
    public <A extends BaseDao> ITaskHandler addTask(ITaskCallback<A> callback, Class<A> clazz, int idleTIme, int callbackTimeout, @Nullable TokenDto token) {
        AteDelegate d = AteDelegate.get();

        if (this.clazz != clazz) {
            throw new RuntimeException("Clazz type of the callback must match.");
        }

        TaskHandler processorContext;
        synchronized (taskHandlers) {
            processorContext = taskHandlers.stream().filter(p -> p.callback == callback).findFirst().orElse(null);
            if (processorContext != null) return processorContext;
        }

        // Add the processor to the subscription list
        processorContext = new TaskHandler(this, clazz, callback)
                .withIdleTime(idleTIme)
                .withCallbackTimeout(callbackTimeout)
                .withToken(token);
        synchronized (taskHandlers) {
            this.taskHandlers.add(processorContext);
        }

        processorContext.start();
        return processorContext;
    }

    @Override
    public <A extends BaseDao> boolean removeTask(ITaskCallback<A> callback, Class<A> clazz) {
        AteDelegate d = AteDelegate.get();

        if (this.clazz != clazz) {
            throw new RuntimeException("Clazz type of the callback must match.");
        }

        synchronized (taskHandlers) {
            for (TaskHandler<T> taskHandler : taskHandlers) {
                if (taskHandler.id().equals(callback.id())) {
                    boolean ret = taskHandlers.remove(taskHandler);
                    taskHandler.stop();
                    return ret;
                }
            }
        }
        return false;
    }

    @Override
    public void feed(MessageDataMetaDto msg) {
        synchronized (taskHandlers) {
            for (TaskHandler<T> taskHandler : this.taskHandlers) {
                taskHandler.feed(msg);
            }
        }
    }

    @Override
    public boolean isEmpty() {
        return this.taskHandlers.isEmpty();
    }

    @Override
    public void clean() {
        synchronized (taskHandlers) {
            List<TaskHandler<T>> toRemove = taskHandlers.stream()
                    .filter(h -> h.isActive() == false)
                    .collect(Collectors.toList());
            for (TaskHandler<T> taskHandler : toRemove) {
                d.debugLogging.logCallbackHook("gc-callback-task", this.partitionKey, this.clazz, null);
                taskHandlers.remove(taskHandler);
                taskHandler.stop();
            }
        }
    }

    @Override
    public void destroyAll() {
        synchronized (taskHandlers) {
            for (TaskHandler<T> taskHandler : taskHandlers.stream().collect(Collectors.toList())) {
                d.debugLogging.logCallbackHook("gc-callback-task", this.partitionKey, this.clazz, null);
                taskHandlers.remove(taskHandler);
                taskHandler.stop();
            }
        }
    }
}
