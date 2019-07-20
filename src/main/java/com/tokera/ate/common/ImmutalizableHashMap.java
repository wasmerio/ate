package com.tokera.ate.common;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.tokera.ate.annotations.YamlTag;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import java.io.Serializable;
import java.util.*;
import java.util.function.BiFunction;
import java.util.function.Function;

@Dependent
@YamlTag("ihashmap")
public class ImmutalizableHashMap<K, V> extends HashMap<K, V> implements Map<K, V>, RandomAccess, Cloneable, Serializable, Immutalizable {
    private static final long serialVersionUID = 462498820763181264L;

    public ImmutalizableHashMap() {
    }

    public ImmutalizableHashMap(Map<? extends K, ? extends V> var1) {
        super(var1);
    }

    public ImmutalizableHashMap(SortedMap<K, ? extends V> var1) {
        super(var1);
    }

    @com.jsoniter.annotation.JsonIgnore
    @JsonIgnore
    private transient boolean _immutable = false;

    @Override
    public @Nullable V put(K var1, V var2) {
        assert this._immutable == false;
        return super.put(var1, var2);
    }

    @Override
    public @Nullable V remove(@Nullable Object var1) {
        assert this._immutable == false;
        return super.remove(var1);
    }

    @Override
    public void putAll(Map<? extends K, ? extends V> var1) {
        assert this._immutable == false;
        super.putAll(var1);
    }

    @Override
    public void clear() {
        assert this._immutable == false;
        super.clear();
    }

    @Override
    public void replaceAll(BiFunction<? super K, ? super V, ? extends V> var1) {
        assert this._immutable == false;
        super.replaceAll(var1);
    }

    @Override
    public V putIfAbsent(K var1, V var2) {
        assert this._immutable == false;
        return super.putIfAbsent(var1, var2);
    }

    @Override
    public boolean remove(Object var1, Object var2) {
        assert this._immutable == false;
        return super.remove(var1, var2);
    }

    @Override
    public boolean replace(K var1, V var2, V var3) {
        assert this._immutable == false;
        return super.replace(var1, var2, var3);
    }

    @Override
    public V replace(K var1, V var2) {
        assert this._immutable == false;
        return super.replace(var1, var2);
    }

    @Override
    public V computeIfAbsent(K var1, Function<? super K, ? extends V> var2) {
        assert this._immutable == false;
        return super.computeIfAbsent(var1, var2);
    }

    @Override
    public V computeIfPresent(K var1, BiFunction<? super K, ? super V, ? extends V> var2) {
        assert this._immutable == false;
        return super.computeIfPresent(var1, var2);
    }

    @Override
    public V compute(K var1, BiFunction<? super K, ? super V, ? extends V> var2) {
        assert this._immutable == false;
        return super.compute(var1, var2);
    }

    @Override
    public V merge(K var1, V var2, BiFunction<? super V, ? super V, ? extends V> var3) {
        assert this._immutable == false;
        return super.merge(var1, var2, var3);
    }

    public void copyFrom(Map<K, V> var1) {
        assert this._immutable == false;
        this.clear();
        this.putAll(var1);
    }

    @Override
    public void immutalize() {
        if (this instanceof CopyOnWrite) {
            ((CopyOnWrite)this).copyOnWrite();
        }
        this._immutable = true;
    }
}
