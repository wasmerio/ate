package com.tokera.ate.io.task;

import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.base.BaseDaoInternal;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.delegates.DebugLoggingDelegate;
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
    ConcurrentHashMap<IPartitionKey, ConcurrentHashMap<String, IHookContext>> lookup
            = new ConcurrentHashMap<>();

    /**
     * Cleans up any dead hooks
     */
    private void clean() {
        for (ConcurrentHashMap<String, IHookContext> map : lookup.values()) {
            for (IHookContext context : map.values()) {
                context.clean();
            }
        }

        lookup.entrySet().removeIf(a ->
        {
            a.getValue().entrySet().removeIf(b -> b.getValue().isEmpty());
            return a.getValue().size() <= 0;
        });
    }

    @SuppressWarnings("unchecked")
    public <T extends BaseDao> void hook(IPartitionKey partitionKey, Class<T> clazz, IHookCallback<T> callback) {
        clean();

        ConcurrentHashMap<String, IHookContext> first
                = lookup.computeIfAbsent(partitionKey, k -> new ConcurrentHashMap<>());
        IHookContext second = first.computeIfAbsent(clazz.getName(), c -> new HookContext<>(partitionKey, clazz));
        second.addHook(callback, clazz);

        d.debugLogging.logCallbackHook("hook", partitionKey, clazz, callback.getClass());

        d.io.warmAndWait(partitionKey);
    }

    public <T extends BaseDao> boolean unhook(IPartitionKey partitionKey, IHookCallback<T> callback, Class<T> clazz) {

        IHookContext context = getContext(partitionKey, clazz.getName());
        if (context.removeHook(callback, clazz)) {
            d.debugLogging.logCallbackHook("unhook", partitionKey, clazz, callback.getClass());
            return true;
        }

        return false;
    }

    public <T extends BaseDao> boolean unhook(IHookCallback<T> callback, Class<T> clazz) {
        boolean ret = false;
        List<IHookContext> contexts = lookup
                .values()
                .stream()
                .map(a -> a.getOrDefault(clazz, null))
                .filter(a -> a != null)
                .collect(Collectors.toList());
        for (IHookContext context : contexts) {
            if (context.removeHook(callback, clazz) == true) {
                d.debugLogging.logCallbackHook("unhook", context.partitionKey(), clazz, callback.getClass());
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

        // Get the clazz name and search for a context thats interested in it
        MessageDataHeaderDto header = data.getHeader();
        String clazzName = header.getPayloadClazzOrThrow();

        // Now get the context and callback
        IHookContext context = getContext(partitionKey, clazzName);
        if (context == null) return;

        MessageDataMetaDto msg = new MessageDataMetaDto(data, meta);
        context.feed(msg);
    }

    @SuppressWarnings("unchecked")
    private <T extends BaseDao> IHookContext getContext(IPartitionKey partitionKey, String clazzName) {
        ConcurrentHashMap<String, IHookContext> first = MapTools.getOrNull(lookup, partitionKey);
        if (first == null) return null;
        return MapTools.getOrNull(first, clazzName);
    }
}
