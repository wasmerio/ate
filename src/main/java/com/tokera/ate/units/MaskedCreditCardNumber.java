package com.tokera.ate.units;

import org.checkerframework.framework.qual.DefaultQualifierInHierarchy;
import org.checkerframework.framework.qual.SubtypeOf;

import java.lang.annotation.ElementType;
import java.lang.annotation.Target;

@CreditCardNumber
@DefaultQualifierInHierarchy
@SubtypeOf(CreditCardNumber.class)
@Target({ElementType.TYPE_USE, ElementType.TYPE_PARAMETER})
public @interface MaskedCreditCardNumber {
}
