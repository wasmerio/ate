package com.tokera.ate.units;

import org.checkerframework.framework.qual.DefaultQualifierInHierarchy;
import org.checkerframework.framework.qual.SubtypeOf;

import javax.validation.constraints.Size;
import java.lang.annotation.ElementType;
import java.lang.annotation.Target;

@Size(min=1)
@PlainText
@DefaultQualifierInHierarchy
@SubtypeOf(PlainText.class)
@Target({ElementType.TYPE_USE, ElementType.TYPE_PARAMETER})
public @interface TextDocument {
}
