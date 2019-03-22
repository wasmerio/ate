/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.scope;

import javax.enterprise.context.NormalScope;
import java.lang.annotation.*;

/**
 *
 * @author John
 */
@Documented
@NormalScope(passivating = false)
@Inherited
@Retention(RetentionPolicy.RUNTIME)
@Target({ElementType.TYPE, ElementType.METHOD, ElementType.FIELD})
public @interface TokenScoped {

}
