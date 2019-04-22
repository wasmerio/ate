package com.tokera.ate.units;

import org.checkerframework.framework.qual.DefaultQualifierInHierarchy;
import org.checkerframework.framework.qual.SubtypeOf;

import java.lang.annotation.ElementType;
import java.lang.annotation.Target;

@Quantity
@Limit
@DefaultQualifierInHierarchy
@SubtypeOf({Quantity.class, Limit.class})
@Target({ElementType.TYPE_USE, ElementType.TYPE_PARAMETER})
public @interface QuantityLimit {
}
