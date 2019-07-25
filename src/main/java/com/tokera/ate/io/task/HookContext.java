package com.tokera.ate.io.task;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.MessageDataMetaDto;
import com.tokera.ate.io.api.*;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.LinkedList;
import java.util.List;
import java.util.stream.Collectors;

/**
 * Represents a partition and class context that callbacks will be invoked under
 */
public class HookContext<T extends BaseDao> implements IHookContext {
    AteDelegate d = AteDelegate.get();

    public final IPartitionKey partitionKey;
    public final Class<T> clazz;
    public final List<Hook<T>> hooks;

    public HookContext(IPartitionKey partitionKey, Class<T> clazz) {
        this.partitionKey = partitionKey;
        this.clazz = clazz;
        this.hooks = new LinkedList<>();
    }

    @Override
    public IPartitionKey partitionKey() {
        return this.partitionKey;
    }

    @Override
    @SuppressWarnings("unchecked")
    public <A extends BaseDao> void addHook(IHookCallback<A> callback, Class<A> clazz) {
        AteDelegate d = AteDelegate.get();

        if (this.clazz != clazz) {
            throw new RuntimeException("Clazz type of the callback must match.");
        }

        synchronized (hooks) {
            Hook context = new Hook(this.partitionKey, callback, clazz, d.currentToken.getTokenOrNull());
            hooks.add(context);
        }
    }

    @Override
    @SuppressWarnings("unchecked")
    public <A extends BaseDao> boolean removeHook(IHookCallback<A> callback, Class<A> clazz) {
        AteDelegate d = AteDelegate.get();

        if (this.clazz != clazz) {
            throw new RuntimeException("Clazz type of the callback must match.");
        }

        synchronized (hooks) {
            for (Hook<T> hook : hooks) {
                if (hook.id().equals(callback.id())) {
                    return hooks.remove(hook);
                }
            }
        }
        return false;
    }

    @Override
    public void feed(MessageDataMetaDto msg) {
        synchronized (hooks) {
            for (Hook<T> hook : this.hooks) {
                hook.feed(msg);
            }
        }
    }

    @Override
    public boolean isEmpty() {
        return this.hooks.isEmpty();
    }

    @Override
    public void clean() {
        synchronized (hooks) {
            List<Hook<T>> toRemove = hooks.stream()
                    .filter(h -> h.isActive() == false)
                    .collect(Collectors.toList());
            for (Hook<T> hook : toRemove) {
                hooks.remove(hook);
            }
        }
    }
}
