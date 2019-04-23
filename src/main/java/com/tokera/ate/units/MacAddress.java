package com.tokera.ate.units;

import org.checkerframework.framework.qual.DefaultQualifierInHierarchy;
import org.checkerframework.framework.qual.SubtypeOf;

import javax.validation.constraints.Pattern;
import java.lang.annotation.ElementType;
import java.lang.annotation.Target;

@Pattern(regexp = "^([0-9a-fA-F]{2}[:-]){5}([0-9a-fA-F]{2})$")
@DefaultQualifierInHierarchy
@SubtypeOf({})
@Target({ElementType.TYPE_USE, ElementType.TYPE_PARAMETER})
public @interface MacAddress {
}
