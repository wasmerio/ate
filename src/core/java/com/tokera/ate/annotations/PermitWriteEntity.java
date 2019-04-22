package com.tokera.ate.annotations;

import com.tokera.ate.dao.base.BaseDao;
import java.lang.annotation.Documented;
import java.lang.annotation.ElementType;
import java.lang.annotation.Inherited;
import java.lang.annotation.Repeatable;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * Allows this REST method to be executed only if it owns a write permission claim to a particular parameter value in
 * its currentRights URL
 * @author John Sharratt (johnathan.sharratt@gmail.com)
 */
@Inherited
@Target(value = {ElementType.TYPE, ElementType.METHOD})
@Retention(value = RetentionPolicy.RUNTIME)
@Documented
@Repeatable(PermitWriteEntities.class)
public @interface PermitWriteEntity {

    String[] name();
    String prefix() default "";
    Class<? extends BaseDao> clazz();
}
