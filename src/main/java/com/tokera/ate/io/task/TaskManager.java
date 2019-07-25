package com.tokera.ate.io.task;

import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.delegates.AteDelegate;
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
        ConcurrentHashMap<Class<? extends BaseDao>, ITaskContext> first
                = lookup.computeIfAbsent(partitionKey, k -> new ConcurrentHashMap<>());
        ITaskContext second = first.computeIfAbsent(clazz, c -> new TaskContext(partitionKey, clazz));
        return second.addTask(callback, clazz, idleTIme, token);
    }

    public <T extends BaseDao> boolean unsubscribe(IPartitionKey partitionKey, ITaskCallback<T> callback, Class<T> clazz) {
        ITaskContext context = getContext(partitionKey, clazz);
        return context.removeTask(callback, clazz);
    }

    public <T extends BaseDao> boolean unsubscribe(ITaskCallback<T> callback, Class<T> clazz) {
        boolean ret = false;
        List<ITaskContext> contexts = lookup
                .values()
                .stream()
                .flatMap(a -> a.values().stream())
                .collect(Collectors.toList());
        for (ITaskContext context : contexts) {
            if (context.removeTask(callback, clazz) == true) {
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
