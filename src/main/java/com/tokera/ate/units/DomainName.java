package com.tokera.ate.units;

import org.checkerframework.framework.qual.DefaultQualifierInHierarchy;
import org.checkerframework.framework.qual.SubtypeOf;

import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;
import java.lang.annotation.ElementType;
import java.lang.annotation.Target;

@Size(min=1, max=512)
@Pattern(regexp = "^([a-z0-9]+(-[a-z0-9]+)*\\.)+[a-z]{2,}$")
@Alias
@DefaultQualifierInHierarchy
@SubtypeOf(Alias.class)
@Target({ElementType.TYPE_USE, ElementType.TYPE_PARAMETER})
public @interface DomainName {
}

