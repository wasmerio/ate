package com.tokera.ate.io.task;

import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageDataHeaderDto;
import com.tokera.ate.dto.msg.MessageDataMetaDto;
import com.tokera.ate.dto.msg.MessageMetaDto;
import com.tokera.ate.io.api.IHookCallback;
import com.tokera.ate.io.api.IHookContext;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.api.ITaskContext;
import com.tokera.ate.scopes.Startup;

import javax.enterprise.context.ApplicationScoped;
import java.util.List;
import java.util.concurrent.ConcurrentHashMap;
import java.util.stream.Collectors;

/**
 * Hook manager used to attach hooks to particular partitions and class types so that Tokera can run an event
 * driver architecture
 */
@Startup
@ApplicationScoped
public class HookManager {
    AteDelegate d = AteDelegate.get();
    ConcurrentHashMap<IPartitionKey, ConcurrentHashMap<Class<? extends BaseDao>, IHookContext>> lookup
            = new ConcurrentHashMap<>();

    /**
     * Cleans up any dead hooks
     */
    private void clean() {
        for (ConcurrentHashMap<Class<? extends BaseDao>, IHookContext> map : lookup.values()) {
            for (IHookContext context : map.values()) {
                context.clean();
            }
        }
    }

    @SuppressWarnings("unchecked")
    public <T extends BaseDao> void hook(IPartitionKey partitionKey, Class<T> clazz, IHookCallback<T> callback) {
        clean();

        ConcurrentHashMap<Class<? extends BaseDao>, IHookContext> first
                = lookup.computeIfAbsent(partitionKey, k -> new ConcurrentHashMap<>());
        IHookContext second = first.computeIfAbsent(clazz, c -> new HookContext<>(partitionKey, clazz));
        second.addHook(callback, clazz);

        d.io.warmAndWait(partitionKey);
    }

    public <T extends BaseDao> boolean unhook(IPartitionKey partitionKey, IHookCallback<T> callback, Class<T> clazz) {
        IHookContext context = getContext(partitionKey, clazz);
        return context.removeHook(callback, clazz);
    }

    public <T extends BaseDao> boolean unhook(IHookCallback<T> callback, Class<T> clazz) {
        boolean ret = false;
        List<IHookContext> contexts = lookup
                .values()
                .stream()
                .flatMap(a -> a.values().stream())
                .collect(Collectors.toList());
        for (IHookContext context : contexts) {
            if (context.removeHook(callback, clazz) == true) {
                ret = true;
            }
        }
        return ret;
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
        IHookContext context = getContext(partitionKey, clazz);
        if (context == null) return;

        MessageDataMetaDto msg = new MessageDataMetaDto(data, meta);
        context.feed(msg);
    }

    @SuppressWarnings("unchecked")
    private <T extends BaseDao> IHookContext getContext(IPartitionKey partitionKey, Class<T> clazz) {
        ConcurrentHashMap<Class<? extends BaseDao>, IHookContext> first = MapTools.getOrNull(lookup, partitionKey);
        if (first == null) return null;
        return MapTools.getOrNull(first, clazz);
    }
}
