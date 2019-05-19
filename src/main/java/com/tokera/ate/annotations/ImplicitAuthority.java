package com.tokera.ate.annotations;

import java.lang.annotation.*;

/**
 * Indicates that this data object field is a domain name that gives implicit write authority
 * @author John Sharratt (johnathan.sharratt@gmail.com)
 */
@Target(value = {ElementType.FIELD})
@Retention(value = RetentionPolicy.RUNTIME)
@Documented
public @interface ImplicitAuthority {
}
