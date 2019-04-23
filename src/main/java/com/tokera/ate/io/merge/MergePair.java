package com.tokera.ate.io.merge;

import org.checkerframework.checker.nullness.qual.Nullable;

public class MergePair<T> {
    public final @Nullable T base;
    public final @Nullable T what;

    public MergePair(@Nullable T base, @Nullable T what) {
        this.base = base;
        this.what = what;
    }
}
