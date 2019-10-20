package com.tokera.ate.io.task;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.msg.MessageDataMetaDto;
import com.tokera.ate.io.api.IHook;
import com.tokera.ate.io.api.IHookCallback;
import com.tokera.ate.io.api.IPartitionKey;

import java.lang.ref.WeakReference;
import java.util.UUID;
import java.util.concurrent.ConcurrentLinkedQueue;

public class Hook<T extends BaseDao> implements IHook
{
    private final UUID id;
    private final IPartitionKey partitionKey;
    private final WeakReference<IHookCallback<T>> callback;
    private final Class<T> clazz;
    private final ConcurrentLinkedQueue<MessageDataMetaDto> toProcess;

    public Hook(IPartitionKey partitionKey, IHookCallback<T> callback, Class<T> clazz) {
        this.id = callback.id();
        this.partitionKey = partitionKey;
        this.callback = new WeakReference<>(callback);
        this.clazz = clazz;
        this.toProcess = new ConcurrentLinkedQueue<>();
    }

    @Override
    public void feed(MessageDataMetaDto msg)
    {
        if (this.isActive() == false) return;

        IHookCallback callback = this.callback.get();
        if (callback != null) {
            callback.onData(msg, this);
        }
    }

    @Override
    public IPartitionKey partitionKey() {
        return this.partitionKey;
    }

    @Override
    public boolean isActive() {
        return this.callback.get() != null;
    }

    @Override
    public UUID id() { return this.id; }
}
