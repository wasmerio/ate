package com.tokera.ate.io.task;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.base.BaseDaoInternal;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.delegates.DebugLoggingDelegate;
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageDataHeaderDto;
import com.tokera.ate.dto.msg.MessageDataMetaDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.api.ITask;
import com.tokera.ate.io.api.ITaskCallback;
import org.apache.commons.lang3.time.DateUtils;
import org.apache.commons.lang3.time.StopWatch;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.jboss.weld.context.bound.BoundRequestContext;

import javax.enterprise.inject.spi.CDI;
import java.lang.ref.WeakReference;
import java.util.*;
import java.util.concurrent.ConcurrentLinkedQueue;

/**
 * Represents the context of a processor to be invoked on callbacks, this object can be used to unsubscribe
 */
public class Task<T extends BaseDao> implements Runnable, ITask {
    public final UUID id;
    public final TaskContext<T> context;
    public final WeakReference<ITaskCallback<T>> callback;
    public final @Nullable TokenDto token;
    public final ConcurrentLinkedQueue<MessageDataMetaDto> toProcess;
    public final Class<T> clazz;
    public final int idleTime;

    private @MonotonicNonNull Thread thread;
    private volatile boolean isRunning = true;
    private Date lastIdle = new Date();

    public Task(TaskContext<T> context, Class<T> clazz, ITaskCallback<T> callback, int idleTime, @Nullable TokenDto token) {
        this.id = callback.id();
        this.context = context;
        this.clazz = clazz;
        this.callback = new WeakReference<>(callback);
        this.token = token;
        this.toProcess = new ConcurrentLinkedQueue<>();
        this.idleTime = idleTime;
    }

    @Override
    public IPartitionKey partitionKey() {
        return context.partitionKey();
    }

    @Override
    public Class<? extends BaseDao> clazz() {
        return clazz;
    }

    @Override
    public @Nullable TokenDto token() {
        return token;
    }

    @Override
    public boolean isActive() {
        return this.callback.get() != null;
    }

    @Override
    public UUID id() { return this.id; }

    public void start() {
        if (this.thread == null) {
            this.thread = new Thread(this);
            this.thread.setDaemon(true);
        }

        this.isRunning = true;
        this.thread.start();
    }

    public void stop() {
        isRunning = false;

        if (this.thread != null) {
            this.thread.interrupt();
        }
    }

    @Override
    public void run()
    {
        boolean doneExisting = false;
        AteDelegate d = AteDelegate.get();

        // Create the bounded request context
        BoundRequestContext boundRequestContext = CDI.current().select(BoundRequestContext.class).get();

        // Enter the main processing loop
        StopWatch timer = new StopWatch();
        timer.start();
        while (isRunning && this.isActive()) {
            try {
                if (doneExisting == false) {
                    invokeSeedKeys(boundRequestContext);
                    invokeInit(boundRequestContext);
                    doneExisting = true;
                }

                invokeTick(boundRequestContext);

                ArrayList<MessageDataMetaDto> msgs = new ArrayList<>();
                for (int n = 0; n < 1000; n++) {
                    if (toProcess.isEmpty()) break;
                    MessageDataMetaDto msg = toProcess.poll();
                    if (msg == null) break;
                    msgs.add(msg);
                }

                if (msgs.size() <= 0)
                {
                    if (DateUtils.addMilliseconds(lastIdle, this.idleTime).before(new Date())) {
                        invokeWarmAndIdle(boundRequestContext);
                        lastIdle = new Date();
                    }

                    synchronized (this.toProcess) {
                        this.toProcess.wait(Math.max(this.idleTime, 1000));
                    }
                }

                invokeMessages(boundRequestContext, msgs);

            } catch (InterruptedException e){
                continue;
            } catch (Throwable ex) {
                d.genericLogger.warn(ex);
            }
        }
    }

    @Override
    public void feed(MessageDataMetaDto msg) {
        if (this.isActive() == false) return;

        this.toProcess.add(msg);
        synchronized (this.toProcess) {
            this.toProcess.notify();
        }
    }

    /**
     * Gathers all the objects in the tree of this particular type and invokes a processor for them
     */
    public void invokeInit(BoundRequestContext boundRequestContext) {
        Task.enterRequestScopeAndInvoke(this.partitionKey(), boundRequestContext, token, () ->
        {
            AteDelegate d = AteDelegate.get();
            d.io.warmAndWait();

            ITaskCallback<T> callback = this.callback.get();
            if (callback != null) callback.onInit(this);
        });
    }

    public void invokeSeedKeys(BoundRequestContext boundRequestContext) {
        Task.enterRequestScopeAndInvoke(this.partitionKey(), boundRequestContext, token, () ->
        {
            AteDelegate d = AteDelegate.get();
            d.io.warm();

            for (PrivateKeyWithSeedDto key : d.currentRights.getRightsRead()) {
                d.io.write(this.partitionKey(), key.key());
            }
            for (PrivateKeyWithSeedDto key : d.currentRights.getRightsWrite()) {
                d.io.write(this.partitionKey(), key.key());
            }
        });
    }

    @SuppressWarnings("unchecked")
    public void invokeMessages(BoundRequestContext boundRequestContext, Iterable<MessageDataMetaDto> msgs) {
        Task.enterRequestScopeAndInvoke(this.partitionKey(), boundRequestContext, token, () ->
        {
            AteDelegate d = AteDelegate.get();
            for (MessageDataMetaDto msg : msgs) {
                try {
                    MessageDataDto data = msg.getData();
                    MessageDataHeaderDto header = data.getHeader();

                    ITaskCallback<T> callback = this.callback.get();
                    if (callback == null) continue;

                    PUUID id = PUUID.from(partitionKey(), header.getIdOrThrow());
                    if (data.hasPayload() == false) {
                        d.debugLogging.logCallbackData("feed-task", id.partition(), id.id(), DebugLoggingDelegate.CallbackDataType.Removed, callback.getClass(), null);
                        callback.onRemove(id, this);
                        continue;
                    }

                    if (d.authorization.canRead(id.partition(), id.id()) == false) {
                        continue;
                    }

                    BaseDao obj = d.dataSerializer.fromDataMessage(partitionKey(), msg, true);
                    if (obj == null || obj.getClass() != clazz) continue;
                    BaseDaoInternal.setPartitionKey(obj, this.partitionKey());
                    BaseDaoInternal.setPreviousVersion(obj, msg.getVersionOrThrow());
                    BaseDaoInternal.setMergesVersions(obj, null);

                    try {
                        d.io.underTransaction(false, () -> {
                            d.io.currentTransaction().cache(id.partition(), obj);

                            if (header.getPreviousVersion() == null) {
                                d.debugLogging.logCallbackData("feed-task", id.partition(), id.id(), DebugLoggingDelegate.CallbackDataType.Created, callback.getClass(), obj);
                                callback.onCreate((T) obj, this);
                            } else {
                                d.debugLogging.logCallbackData("feed-task", id.partition(), id.id(), DebugLoggingDelegate.CallbackDataType.Update, callback.getClass(), obj);
                                callback.onUpdate((T) obj, this);
                            }
                        });
                    } catch (Throwable ex) {
                        d.io.underTransaction(false, () -> {
                            callback.onException((T)obj, this, ex);
                        });
                    } finally {
                        d.io.currentTransaction().clear();
                    }
                } catch (Throwable ex) {
                    d.genericLogger.warn(ex);
                }
            }
        });
    }

    public void invokeTick(BoundRequestContext boundRequestContext) {
        Task.enterRequestScopeAndInvoke(this.partitionKey(), boundRequestContext, token, () -> {
            ITaskCallback<T> callback = this.callback.get();
            if (callback != null) callback.onTick(this);
        });
    }

    public void invokeWarmAndIdle(BoundRequestContext boundRequestContext) {
        AteDelegate d = AteDelegate.get();
        Task.enterRequestScopeAndInvoke(this.partitionKey(), boundRequestContext, token, () -> {
            d.io.warm(partitionKey());
            ITaskCallback<T> callback = this.callback.get();
            if (callback != null) callback.onIdle(this);
        });
    }



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

        Map<String, Object> requestDataStore = new TreeMap<>();
        boundRequestContext.associate(requestDataStore);
        try {
            boundRequestContext.activate();
            try {
                // Publish the token but skip the validation as we already trust the token
                if (token != null) {
                    d.currentToken.setSkipValidation(true);
                    d.currentToken.setPerformedValidation(true);
                    d.currentToken.publishToken(token);
                }

                // Run the stuff under this scope context
                d.requestContext.pushPartitionKey(partitionKey);
                try {
                    callback.run();
                } finally {
                    d.requestContext.popPartitionKey();
                }

                // Invoke the merge
                d.io.flushAll();
            } finally {
                boundRequestContext.invalidate();
                boundRequestContext.deactivate();
            }
        } catch (Throwable ex) {
            d.genericLogger.warn(ex);
            if (ex instanceof InterruptedException) throw ex;
        } finally {
            boundRequestContext.dissociate(requestDataStore);
        }
    }
}
