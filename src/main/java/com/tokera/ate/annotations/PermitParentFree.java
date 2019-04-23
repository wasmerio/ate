/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.annotations;

import java.lang.annotation.*;

/**
 * Allows this data object to exist without being attached to a parent object
 * @author John Sharratt (johnathan.sharratt@gmail.com)
 */
@Target(value = {ElementType.TYPE})
@Retention(value = RetentionPolicy.RUNTIME)
@Documented
public @interface PermitParentFree {
}