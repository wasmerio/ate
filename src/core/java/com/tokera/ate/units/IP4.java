package com.tokera.ate.units;

import org.checkerframework.framework.qual.DefaultQualifierInHierarchy;
import org.checkerframework.framework.qual.SubtypeOf;

import javax.validation.constraints.Pattern;
import java.lang.annotation.ElementType;
import java.lang.annotation.Target;

@Pattern(regexp = "^(([01]?\\d\\d?|2[0-4]\\d|25[0-5])\\.){3}([01]?\\d\\d?|2[0-4]\\d|25[0-5])$")
@DefaultQualifierInHierarchy
@SubtypeOf(IP.class)
@Target({ElementType.TYPE_USE, ElementType.TYPE_PARAMETER})
public @interface IP4 {
}
