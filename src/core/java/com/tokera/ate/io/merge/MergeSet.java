package com.tokera.ate.io.merge;

import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.LinkedList;
import java.util.function.Function;

public class MergeSet<T> {
    public final @Nullable T first;
    public final @Nullable T base;
    public final LinkedList<@Nullable T> stream = new LinkedList<>();

    public MergeSet(@Nullable T first, @Nullable T base) {
        this.first = first;
        this.base = base;
    }

    public void add(@Nullable T right) {
        stream.add(right);
    }

    public <B> MergeSet<B> convert(Function<@Nullable T, @Nullable B> convert) {
        MergeSet<B> ret = new MergeSet<>(convert.apply(first), convert.apply(base));
        for (T right : stream) {
            ret.stream.add(convert.apply(right));
        }
        return ret;
    }
}
