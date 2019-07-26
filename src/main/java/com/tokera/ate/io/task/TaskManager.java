package com.tokera.ate.io.task;

import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.delegates.DebugLoggingDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageDataHeaderDto;
import com.tokera.ate.dto.msg.MessageDataMetaDto;
import com.tokera.ate.dto.msg.MessageMetaDto;
import com.tokera.ate.io.api.*;
import com.tokera.ate.scopes.Startup;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.jboss.weld.context.bound.BoundRequestContext;

import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.inject.spi.CDI;
import java.util.List;
import java.util.concurrent.ConcurrentHashMap;
import java.util.stream.Collectors;

/**
 * Task manager used to attach tasks to particular partitions and class types so that Tokera can run an event
 * driver architecture
 */
@Startup
@ApplicationScoped
public class TaskManager {
    AteDelegate d = AteDelegate.get();
    ConcurrentHashMap<IPartitionKey, ConcurrentHashMap<Class<? extends BaseDao>, ITaskContext>> lookup
            = new ConcurrentHashMap<>();

    public static int DEFAULT_IDLE_TIME = 1000;

    /**
     * Cleans up any dead hooks
     */
    private void clean() {
        for (ConcurrentHashMap<Class<? extends BaseDao>, ITaskContext> map : lookup.values()) {
            for (ITaskContext context : map.values()) {
                context.clean();
            }
        }

        lookup.entrySet().removeIf(a ->
        {
            a.getValue().entrySet().removeIf(b -> b.getValue().isEmpty());
            return a.getValue().size() <= 0;
        });
    }

    public <T extends BaseDao> ITask subscribe(IPartitionKey partitionKey, Class<T> clazz, ITaskCallback<T> callback) {
        return subscribe(partitionKey, clazz, callback, DEFAULT_IDLE_TIME);
    }

    public <T extends BaseDao> ITask subscribe(IPartitionKey partitionKey, Class<T> clazz, ITaskCallback<T> callback, int idleTime) {
        TokenDto token = d.currentToken.getTokenOrNull();
        return subscribe(partitionKey, clazz, callback, idleTime, token);
    }

    public <T extends BaseDao> ITask subscribe(IPartitionKey partitionKey, Class<T> clazz, ITaskCallback<T> callback, @Nullable TokenDto token) {
        return subscribe(partitionKey, clazz, callback, DEFAULT_IDLE_TIME, token);
    }

    @SuppressWarnings("unchecked")
    public <T extends BaseDao> ITask subscribe(IPartitionKey partitionKey, Class<T> clazz, ITaskCallback<T> callback, int idleTIme, @Nullable TokenDto token) {
        clean();

        ConcurrentHashMap<Class<? extends BaseDao>, ITaskContext> first
                = lookup.computeIfAbsent(partitionKey, k -> new ConcurrentHashMap<>());
        ITaskContext second = first.computeIfAbsent(clazz, c -> new TaskContext(partitionKey, clazz));

        ITask ret = second.addTask(callback, clazz, idleTIme, token);
        d.debugLogging.logCallbackHook("subscribe", partitionKey, clazz, callback.getClass(), null);
        return ret;
    }

    public <T extends BaseDao> boolean unsubscribe(IPartitionKey partitionKey, ITaskCallback<T> callback, Class<T> clazz) {
        ITaskContext context = getContext(partitionKey, clazz);
        if (context.removeTask(callback, clazz) == true) {
            d.debugLogging.logCallbackHook("unsubscribe", context.partitionKey(), clazz, callback.getClass(), null);
            return true;
        }
        return false;
    }

    public <T extends BaseDao> boolean unsubscribe(ITaskCallback<T> callback, Class<T> clazz) {
        boolean ret = false;
        List<ITaskContext> contexts = lookup
                .values()
                .stream()
                .map(a -> a.getOrDefault(clazz, null))
                .filter(a -> a != null)
                .collect(Collectors.toList());
        for (ITaskContext context : contexts) {
            if (context.removeTask(callback, clazz) == true) {
                d.debugLogging.logCallbackHook("unsubscribe", context.partitionKey(), clazz, callback.getClass(), null);
                ret = true;
            }
        }
        return ret;
    }

    @SuppressWarnings("unchecked")
    private <T extends BaseDao> ITaskContext getContext(IPartitionKey partitionKey, Class<T> clazz) {
        ConcurrentHashMap<Class<? extends BaseDao>, ITaskContext> first = MapTools.getOrNull(lookup, partitionKey);
        if (first == null) return null;
        return MapTools.getOrNull(first, clazz);
    }

    /**
     * Callback invoked whenever a data object changes or is created in this context
     */
    public void feed(IPartitionKey partitionKey, MessageDataDto data, MessageMetaDto meta) {
        if (lookup.containsKey(partitionKey) == false) return;

        // Find the type of object this is
        MessageDataHeaderDto header = data.getHeader();
        String clazzName = header.getPayloadClazzOrThrow();
        Class<BaseDao> clazz = d.serializableObjectsExtension.findClass(clazzName, BaseDao.class);

        // Now get the context and callback
        ITaskContext context = getContext(partitionKey, clazz);
        if (context == null) return;

        MessageDataMetaDto msg = new MessageDataMetaDto(data, meta);
        context.feed(msg);
    }
}
