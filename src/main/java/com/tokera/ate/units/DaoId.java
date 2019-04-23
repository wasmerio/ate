package com.tokera.ate.units;

import com.tokera.ate.dao.base.BaseDao;
import org.checkerframework.framework.qual.DefaultQualifierInHierarchy;
import org.checkerframework.framework.qual.SubtypeOf;

import java.lang.annotation.ElementType;
import java.lang.annotation.Target;

@DefaultQualifierInHierarchy
@SubtypeOf({})
@Target({ElementType.TYPE_USE, ElementType.TYPE_PARAMETER})
public @interface DaoId {
    Class<? extends BaseDao> value() default BaseDao.class;
}
