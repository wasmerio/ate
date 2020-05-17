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
import com.tokera.ate.io.api.*;
import com.tokera.ate.scopes.Startup;

import javax.enterprise.context.ApplicationScoped;
import java.util.LinkedList;
import java.util.List;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentSkipListSet;
import java.util.concurrent.atomic.AtomicReference;
import java.util.concurrent.locks.ReentrantReadWriteLock;
import java.util.stream.Collectors;

/**
 * Hook manager used to attach hooks to particular partitions and class types so that Tokera can run an event
 * driver architecture
 */
@Startup
@ApplicationScoped
public class HookManager {
    private final AteDelegate d = AteDelegate.get();

    private final ReentrantReadWriteLock allLock = new ReentrantReadWriteLock();
    private final LinkedList<IHookFeed> all = new LinkedList<>();

    private final ConcurrentHashMap<IPartitionKey, ConcurrentHashMap<String, IHookContext>> lookup
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

        lookup.compute(partitionKey, (k, map) ->
        {
            if (map == null) map = new ConcurrentHashMap<>();
            map.compute(clazz.getName(), (c, ctx) ->
            {
                if (ctx == null) ctx = new HookContext<>(partitionKey, clazz);
                ctx.addHook(callback, clazz);
                return ctx;
            });
            return map;
        });

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
     * Hooks all data notifications for all partitions
     * @param feed callback that will be invoked
     */
    public void hookAll(IHookFeed feed) {
        ReentrantReadWriteLock.WriteLock lockScope = allLock.writeLock();
        lockScope.lock();
        try {
            this.all.add(feed);
        } finally {
            lockScope.unlock();
        }
    }

    /**
     * Unhooks data notifications for a particular callback function
     * @param feed callback that is already hooked
     */
    public void unhookAll(IHookFeed feed) {
        ReentrantReadWriteLock.WriteLock lockScope = allLock.writeLock();
        lockScope.lock();
        try {
            this.all.remove(feed);
        } finally {
            lockScope.unlock();
        }
    }

    /**
     * Callback invoked whenever a data object changes or is created in this context
     */
    public void feed(IPartitionKey partitionKey, MessageDataDto data, MessageMetaDto meta)
    {
        MessageDataMetaDto msg = new MessageDataMetaDto(data, meta);

        ReentrantReadWriteLock.ReadLock lockScope = allLock.readLock();
        lockScope.lock();
        try {
            for (IHookFeed feed : this.all) {
                feed.feed(partitionKey, msg);
            }
        } finally {
            lockScope.unlock();;
        }

        if (lookup.containsKey(partitionKey) == false) return;

        // Get the clazz name and search for a context thats interested in it
        MessageDataHeaderDto header = data.getHeader();
        String clazzName = header.getPayloadClazzOrThrow();

        // Now get the context and callback
        IHookContext context = getContext(partitionKey, clazzName);
        if (context == null) return;

        context.feed(partitionKey, msg);
    }

    @SuppressWarnings("unchecked")
    private <T extends BaseDao> IHookContext getContext(IPartitionKey partitionKey, String clazzName) {
        ConcurrentHashMap<String, IHookContext> first = MapTools.getOrNull(lookup, partitionKey);
        if (first == null) return null;
        return MapTools.getOrNull(first, clazzName);
    }
}
