package com.tokera.ate.annotations;

import java.lang.annotation.Documented;
import java.lang.annotation.ElementType;
import java.lang.annotation.Inherited;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

import com.tokera.ate.dao.enumerations.UserRole;

/**
 * Allows this REST method to execute only if it contains a claim for a particular user role
 * @author John Sharratt (johnathan.sharratt@gmail.com)
 */
@Inherited
@Target(value = {ElementType.TYPE, ElementType.METHOD})
@Retention(value = RetentionPolicy.RUNTIME)
@Documented
public @interface PermitUserRole {

    UserRole[] value() default UserRole.ANYTHING;
}
