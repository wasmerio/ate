package com.tokera.ate.units;

import org.checkerframework.checker.fenum.qual.FenumTop;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.checkerframework.framework.qual.SubtypeOf;

import java.lang.annotation.*;

@Nullable
@Documented
@Retention(RetentionPolicy.RUNTIME)
@Target({ElementType.TYPE_USE, ElementType.TYPE_PARAMETER})
@SubtypeOf(FenumTop.class)
public @interface LinuxCmd {
}
