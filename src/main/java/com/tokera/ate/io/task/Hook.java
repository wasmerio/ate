package com.tokera.ate.io.task;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.delegates.DebugLoggingDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageDataHeaderDto;
import com.tokera.ate.dto.msg.MessageDataMetaDto;
import com.tokera.ate.io.api.IHook;
import com.tokera.ate.io.api.IHookCallback;
import com.tokera.ate.io.api.IPartitionKey;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.jboss.weld.context.bound.BoundRequestContext;

import javax.enterprise.inject.spi.CDI;
import java.lang.ref.WeakReference;
import java.util.Map;
import java.util.TreeMap;
import java.util.UUID;
import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.concurrent.Executor;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicBoolean;

public class Hook<T extends BaseDao> implements IHook {
    private final AteDelegate d = AteDelegate.get();

    private static final ExecutorService executor;
    static {
        executor = Executors.newCachedThreadPool();
    }

    private final UUID id;
    private final IPartitionKey partitionKey;
    private final WeakReference<IHookCallback<T>> callback;
    private final Class<T> clazz;
    private final TokenDto token;
    private final ConcurrentLinkedQueue<MessageDataMetaDto> toProcess;
    private final AtomicBoolean woken = new AtomicBoolean(false);

    public Hook(IPartitionKey partitionKey, IHookCallback<T> callback, Class<T> clazz, TokenDto token) {
        this.id = callback.id();
        this.partitionKey = partitionKey;
        this.callback = new WeakReference<>(callback);
        this.clazz = clazz;
        this.token = token;
        this.toProcess = new ConcurrentLinkedQueue<>();
    }

    private void run()
    {
        while (woken.compareAndSet(true, false)) {
            for (;;) {
                if (this.toProcess.isEmpty()) break;
                MessageDataMetaDto msg = toProcess.poll();
                if (msg == null) break;

                BoundRequestContext boundRequestContext = CDI.current().select(BoundRequestContext.class).get();
                Hook.enterRequestScopeAndInvoke(this.partitionKey, boundRequestContext, this.token, () ->
                {
                    try {
                        MessageDataDto data = msg.getData();
                        MessageDataHeaderDto header = data.getHeader();

                        IHookCallback<T> callback = this.callback.get();
                        if (callback == null) return;

                        PUUID id = PUUID.from(partitionKey, header.getIdOrThrow());
                        if (data.hasPayload() == false) {
                            d.debugLogging.logCallbackData("feed-hook", id.partition(), id.id(), DebugLoggingDelegate.CallbackDataType.Removed, callback.getClass(), null);
                            callback.onRemove(id, this);
                            return;
                        }

                        if (AteDelegate.get().authorization.canRead(partitionKey, header.getIdOrThrow()) == false) {
                            return;
                        }

                        BaseDao obj = d.dataSerializer.fromDataMessage(partitionKey, msg, true);
                        if (obj == null || obj.getClass() != clazz) return;

                        if (header.getPreviousVersion() == null) {
                            d.debugLogging.logCallbackData("feed-hook", id.partition(), id.id(), DebugLoggingDelegate.CallbackDataType.Created, callback.getClass(), obj);
                            callback.onData((T) obj, this);
                        } else {
                            d.debugLogging.logCallbackData("feed-hook", id.partition(), id.id(), DebugLoggingDelegate.CallbackDataType.Update, callback.getClass(), obj);
                            callback.onData((T) obj, this);
                        }
                    } catch (Throwable ex) {
                        d.genericLogger.warn(ex);
                    }
                });
            }
        }
    }

    @Override
    @SuppressWarnings("unchecked")
    public void feed(MessageDataMetaDto msg)
    {
        if (this.isActive() == false) return;

        this.toProcess.add(msg);
        if (woken.compareAndSet(false, true)) {
            executor.submit(this::run);
        }
    }

    @Override
    public IPartitionKey partitionKey() {
        return this.partitionKey;
    }

    @Override
    public @Nullable TokenDto token() {
        return this.token;
    }

    @Override
    public boolean isActive() {
        return this.callback.get() != null;
    }

    @Override
    public UUID id() { return this.id; }

    /**
     * Enters a fake request scope and brings the token online so that the callback will
     * @param token
     * @param callback
     */
    public static void enterRequestScopeAndInvoke(IPartitionKey partitionKey, BoundRequestContext boundRequestContext, @Nullable TokenDto token, Runnable callback) {
        AteDelegate d = AteDelegate.get();
        if (boundRequestContext.isActive()) {
            throw new RuntimeException("Nested request context are not currently supported.");
        }

        synchronized (token) {
            Map<String, Object> requestDataStore = new TreeMap<>();
            boundRequestContext.associate(requestDataStore);
            try {
                boundRequestContext.activate();
                try {
                    // Publish the token but skip the validation as we already trust the token
                    d.currentToken.setSkipValidation(true);
                    d.currentToken.setPerformedValidation(true);
                    d.currentToken.publishToken(token);

                    // Run the stuff under this scope context
                    d.requestContext.pushPartitionKey(partitionKey);
                    try {
                        callback.run();
                    } finally {
                        d.requestContext.popPartitionKey();
                    }

                    // Invoke the merge
                    d.io.mergeDeferred();
                } finally {
                    boundRequestContext.invalidate();
                    boundRequestContext.deactivate();
                }
            } catch (Throwable ex) {
                d.genericLogger.warn(ex);
            } finally {
                boundRequestContext.dissociate(requestDataStore);
            }
        }
    }
}
