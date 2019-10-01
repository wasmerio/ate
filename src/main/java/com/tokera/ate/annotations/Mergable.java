package com.tokera.ate.annotations;

import java.lang.annotation.*;

/**
 * Indicates that the data object has public rights if it is not yet claimed by someone
 * i.e. The first one who claims this public key will have rights to it.
 * @author John Sharratt (johnathan.sharratt@gmail.com)
 */
@Target(value = {ElementType.TYPE})
@Retention(value = RetentionPolicy.RUNTIME)
@Documented
public @interface Mergable {
}
