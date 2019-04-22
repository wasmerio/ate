package com.tokera.ate.common;

import com.fasterxml.jackson.annotation.JsonIgnore;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.io.Serializable;
import java.util.*;

public class ImmutalizableHashSet<E> extends HashSet<E> implements Set<E>, Cloneable, Serializable, Immutalizable {
    static final long serialVersionUID = -4024744406713321674L;

    public ImmutalizableHashSet() {
    }

    public ImmutalizableHashSet(Collection<? extends E> var1) {
        super(var1);
    }

    @JsonIgnore
    private transient boolean _immutable = false;

    @Override
    public boolean add(E var1) {
        assert this._immutable == false;
        return super.add(var1);
    }

    @Override
    public boolean remove(@Nullable Object var1) {
        assert this._immutable == false;
        return super.remove(var1);
    }

    @Override
    public boolean addAll(Collection<? extends E> var1) {
        assert this._immutable == false;
        return super.addAll(var1);
    }

    @Override
    public boolean retainAll(Collection<?> var1) {
        assert this._immutable == false;
        return super.retainAll(var1);
    }

    @Override
    public boolean removeAll(Collection<?> var1) {
        assert this._immutable == false;
        return super.removeAll(var1);
    }

    @Override
    public void clear() {
        assert this._immutable == false;
        super.clear();
    }

    public void copyFrom(Collection<? extends E> var1) {
        assert this._immutable == false;
        this.clear();
        this.addAll(var1);
    }

    @Override
    public void immutalize() {
        this._immutable = true;
    }
}
