package com.tokera.ate.common;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.tokera.ate.annotations.YamlTag;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import java.io.Serializable;
import java.util.*;
import java.util.function.UnaryOperator;

@Dependent
@YamlTag("iarraylist")
public class ImmutalizableArrayList<E> extends ArrayList<E> implements List<E>, RandomAccess, Cloneable, Serializable, Immutalizable {
    private static final long serialVersionUID = 4683452581122892184L;

    public ImmutalizableArrayList() {
    }

    public ImmutalizableArrayList(Collection<? extends E> var1) {
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
    public boolean addAll(int var1, Collection<? extends E> var2) {
        assert this._immutable == false;
        return super.addAll(var1, var2);
    }

    @Override
    public boolean removeAll(Collection<?> var1) {
        assert this._immutable == false;
        return super.removeAll(var1);
    }

    @Override
    public boolean retainAll(Collection<?> var1) {
        assert this._immutable == false;
        return super.retainAll(var1);
    }

    @Override
    public void replaceAll(UnaryOperator<E> var1) {
        assert this._immutable == false;
        super.replaceAll(var1);
    }

    @Override
    public void sort(Comparator<? super E> var1) {
        assert this._immutable == false;
        super.sort(var1);
    }

    @Override
    public void clear() {
        assert this._immutable == false;
        super.clear();
    }

    @Override
    public E set(int var1, E var2) {
        assert this._immutable == false;
        return super.set(var1, var2);
    }

    @Override
    public void add(int var1, E var2) {
        assert this._immutable == false;
        super.add(var1, var2);
    }

    @Override
    public E remove(int var1) {
        assert this._immutable == false;
        return super.remove(var1);
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
