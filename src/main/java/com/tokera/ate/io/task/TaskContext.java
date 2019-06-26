package com.tokera.ate.io.task;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.MessageDataMetaDto;
import com.tokera.ate.io.api.*;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.jboss.weld.context.bound.BoundRequestContext;

import java.util.LinkedList;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.stream.Collectors;

/**
 * Represents a partition and class context that callbacks will be invoked under
 */
public class TaskContext<T extends BaseDao> implements ITaskContext {
    AteDelegate d = AteDelegate.get();

    public final IPartitionKey partitionKey;
    public final Class<T> clazz;
    public final List<Task<T>> tasks;
    public final BoundRequestContext boundRequestContext;

    public TaskContext(IPartitionKey partitionKey, Class<T> clazz, BoundRequestContext boundRequestContext) {
        this.partitionKey = partitionKey;
        this.clazz = clazz;
        this.tasks = new LinkedList<>();
        this.boundRequestContext = boundRequestContext;
    }

    @Override
    public IPartitionKey partitionKey() {
        return this.partitionKey;
    }

    @Override
    @SuppressWarnings("unchecked")
    public <A extends BaseDao> ITask addTask(ITaskCallback<A> callback, Class<A> clazz) {
        AteDelegate d = AteDelegate.get();

        if (this.clazz != clazz) {
            throw new RuntimeException("Clazz type of the callback must match.");
        }

        Task processorContext;
        synchronized (tasks) {
            processorContext = tasks.stream().filter(p -> p.callback == callback).findFirst().orElse(null);
            if (processorContext != null) return processorContext;
        }

        // Add the processor to the subscription list
        TokenDto token = d.currentToken.getTokenOrNull();
        processorContext = new Task(this, clazz, callback, token);
        synchronized (tasks) {
            this.tasks.add(processorContext);
        }

        processorContext.start();
        return processorContext;
    }

    @Override
    public boolean removeTask(ITask task) {
        Task<T> ret;
        synchronized (tasks) {
            ret = this.tasks.stream().filter(t -> t == task).findFirst().orElse(null);
            if (ret == null) return false;
        }
        ret.stop();
        return true;
    }

    /**
     * Enters a fake request scope and brings the token online so that the callback will
     * @param token
     * @param callback
     */
    public void enterRequestScopeAndInvoke(@Nullable TokenDto token, Runnable callback) {
        if (boundRequestContext.isActive()) {
            throw new RuntimeException("Nested request context are not currently supported.");
        }

        synchronized (token) {
            Map<String, Object> requestDataStore = new TreeMap<>();
            boundRequestContext.associate(requestDataStore);
            try {
                boundRequestContext.activate();

                // Publish the token but skip the validation as we already trust the token
                d.currentToken.setPerformedValidation(true);
                d.currentToken.publishToken(token);

                // Run the stuff under this scope context
                callback.run();

                boundRequestContext.invalidate();
                boundRequestContext.deactivate();
            } catch (Throwable ex) {
                d.genericLogger.warn(ex);
            } finally {
                boundRequestContext.dissociate(requestDataStore);
            }
        }
    }

    @Override
    public void feed(MessageDataMetaDto msg) {
        synchronized (tasks) {
            for (Task<T> context : this.tasks) {
                context.add(msg);
            }
        }
    }

    @Override
    public List<ITask> tasks() {
        return this.tasks.stream()
                .collect(Collectors.toList());
    }
}
