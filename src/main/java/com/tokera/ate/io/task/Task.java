package com.tokera.ate.io.task;

import com.tokera.ate.common.ConcurrentStack;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.api.ITask;
import com.tokera.ate.io.api.ITaskCallback;
import org.apache.commons.lang.time.StopWatch;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.jboss.weld.context.bound.BoundRequestContext;

import javax.enterprise.inject.spi.CDI;
import java.util.ArrayList;
import java.util.Map;
import java.util.TreeMap;

/**
 * Represents the context of a processor to be invoked on callbacks, this object can be used to unsubscribe
 */
public class Task<T extends BaseDao> implements Runnable, ITask {
    public final TaskContext<T> context;
    public final ITaskCallback<T> callback;
    public final @Nullable TokenDto token;
    public final ConcurrentStack<MessageDataMetaDto> toProcess;
    public final Class<T> clazz;

    private @MonotonicNonNull Thread thread;
    private volatile boolean isRunning = true;

    public Task(TaskContext<T> context, Class<T> clazz, ITaskCallback<T> callback, @Nullable TokenDto token) {
        this.context = context;
        this.clazz = clazz;
        this.callback = callback;
        this.token = token;
        this.toProcess = new ConcurrentStack<>();
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
    public ITaskCallback<T> callback() {
        return callback;
    }

    @Override
    public @Nullable TokenDto token() {
        return token;
    }

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
        while (isRunning) {
            try {
                if (doneExisting == false) {
                    invokeSeedKeys(boundRequestContext);
                    invokeInit(boundRequestContext);
                    doneExisting = true;
                }

                invokeTick(boundRequestContext);

                ArrayList<MessageDataMetaDto> msgs = new ArrayList<>();
                for (int n = 0; n < 1000; n++) {
                    MessageDataMetaDto msg = toProcess.pop();
                    if (msg == null) break;
                    msgs.add(msg);
                }

                if (msgs.size() <= 0) {
                    invokeWarmAndIdle(boundRequestContext);

                    synchronized (this.toProcess) {
                        this.toProcess.wait(1000);
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

    public void add(MessageDataMetaDto msg) {
        this.toProcess.push(msg);
        synchronized (this.toProcess) {
            this.toProcess.notify();
        }
    }

    /**
     * Gathers all the objects in the tree of this particular type and invokes a processor for them
     */
    public void invokeInit(BoundRequestContext boundRequestContext) {
        enterRequestScopeAndInvoke(boundRequestContext, token, () ->
        {
            AteDelegate d = AteDelegate.get();
            callback.onInit(d.io.getAll(clazz), this);
        });
    }

    public void invokeSeedKeys(BoundRequestContext boundRequestContext) {
        enterRequestScopeAndInvoke(boundRequestContext, token, () ->
        {
            AteDelegate d = AteDelegate.get();
            for (MessagePublicKeyDto key : d.currentRights.getRightsRead()) {
                d.io.merge(this.partitionKey(), key);
            }
            for (MessagePublicKeyDto key : d.currentRights.getRightsWrite()) {
                d.io.merge(this.partitionKey(), key);
            }
        });
    }

    @SuppressWarnings("unchecked")
    public void invokeMessages(BoundRequestContext boundRequestContext, Iterable<MessageDataMetaDto> msgs) {
        enterRequestScopeAndInvoke(boundRequestContext, token, () ->
        {
            AteDelegate d = AteDelegate.get();
            for (MessageDataMetaDto msg : msgs) {
                try {
                    MessageDataDto data = msg.getData();
                    MessageDataHeaderDto header = data.getHeader();
                    if (data.hasPayload() == false) {
                        callback.onRemove(PUUID.from(partitionKey(), header.getIdOrThrow()), this);
                        continue;
                    }

                    if (d.authorization.canRead(context.partitionKey(), header.getIdOrThrow()) == false) {
                        continue;
                    }

                    BaseDao obj = d.dataSerializer.fromDataMessage(partitionKey(), msg, true);
                    if (obj == null || obj.getClass() != clazz) continue;
                    callback.onData((T)obj, this);
                } catch (Throwable ex) {
                    d.genericLogger.warn(ex);
                }
            }
        });
    }

    public void invokeTick(BoundRequestContext boundRequestContext) {
        enterRequestScopeAndInvoke(boundRequestContext, token, () -> callback.onTick(this));
    }

    public void invokeWarmAndIdle(BoundRequestContext boundRequestContext) {
        AteDelegate d = AteDelegate.get();
        enterRequestScopeAndInvoke(boundRequestContext, token, () -> {
            d.io.warm(partitionKey());
            callback.onIdle(this);
        });
    }

    /**
     * Enters a fake request scope and brings the token online so that the callback will
     * @param token
     * @param callback
     */
    public void enterRequestScopeAndInvoke(BoundRequestContext boundRequestContext, @Nullable TokenDto token, Runnable callback) {
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
                    callback.run();

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
