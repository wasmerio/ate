package com.tokera.ate.common;

import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.concurrent.atomic.AtomicReference;
import java.util.function.Supplier;

/**
 * Multi-thread safe stack of generic objects. Uses atomic references to maintain the stack using compare-and-set
 * operations on the object references.
 * @param <T> Type of objects contained within this stack
 */
public class ConcurrentStack<T> {
    AtomicReference<@Nullable Item<T>> head = new AtomicReference<>();

    private class Item<T> {
        public final T payload;
        public @Nullable Item<T> next;

        public Item(T item) {
            this.payload = item;
        }
    }

    public @Nullable T pop() {
        Item<T> last;
        Item<T> next;
        do {
            last = head.get();
            if (last == null) return null;
            next = last.next;
        } while (!head.compareAndSet(last, next));
        return last.payload;
    }

    public T popOrInvoke(Supplier<T> callback) {
        T ret = pop();
        if (ret != null) return ret;
        return callback.get();
    }

    public void push(T item) {
        Item<T> next = new Item<T>(item);
        Item<T> last;
        do {
            last = head.get();
            next.next = last;
        } while (!head.compareAndSet(last, next));
    }
}