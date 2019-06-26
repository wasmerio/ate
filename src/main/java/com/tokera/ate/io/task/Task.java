package com.tokera.ate.io.task;

import com.tokera.ate.common.ConcurrentStack;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageDataHeaderDto;
import com.tokera.ate.dto.msg.MessageDataMetaDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.api.ITask;
import com.tokera.ate.io.api.ITaskCallback;
import org.apache.commons.lang.time.StopWatch;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.ArrayList;

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
        AteDelegate d = AteDelegate.get();

        // Invoke any existing objects after its been added so that race conditions are avoided
        invokeExisting();

        // Enter the main processing loop
        StopWatch timer = new StopWatch();
        timer.start();
        while (isRunning) {
            try {
                invokeTick();

                ArrayList<MessageDataMetaDto> msgs = new ArrayList<>();
                for (int n = 0; n < 1000; n++) {
                    MessageDataMetaDto msg = toProcess.pop();
                    if (msg == null) break;
                    msgs.add(msg);
                }

                if (msgs.size() <= 0) {
                    invokeWarm();

                    synchronized (this.toProcess) {
                        this.toProcess.wait(1000);
                    }
                }

                invokeMessages(msgs);

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
    public void invokeExisting() {
        context.enterRequestScopeAndInvoke(token, () ->
        {
            AteDelegate d = AteDelegate.get();
            for (T obj : AteDelegate.get().io.getAll(clazz)) {
                try {
                    callback.onInit(obj, this);
                } catch (Throwable ex) {
                    d.genericLogger.warn(ex);
                }
            }
        });
    }

    @SuppressWarnings("unchecked")
    public void invokeMessages(Iterable<MessageDataMetaDto> msgs) {
        context.enterRequestScopeAndInvoke(token, () ->
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

                    if (d.authorization.canRead(context.partitionKey(), header.getIdOrThrow(), header.getParentId()) == false) {
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

    public void invokeTick() {
        context.enterRequestScopeAndInvoke(token, () -> callback.onTick(this));
    }

    public void invokeWarm() {
        AteDelegate d = AteDelegate.get();
        context.enterRequestScopeAndInvoke(token, () -> d.io.warm(partitionKey()));
    }
}
