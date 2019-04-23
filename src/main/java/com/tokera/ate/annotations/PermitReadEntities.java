package com.tokera.ate.annotations;

import java.lang.annotation.Documented;
import java.lang.annotation.ElementType;
import java.lang.annotation.Inherited;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * Allows this REST method to be executed only if it owns read permission claims to a particular parameter values in the
 * currentRights URL
 * @author John Sharratt (johnathan.sharratt@gmail.com)
 */
@Inherited
@Target(value = {ElementType.TYPE, ElementType.METHOD})
@Retention(value = RetentionPolicy.RUNTIME)
@Documented
public @interface PermitReadEntities {

    PermitReadEntity[] value();
}
