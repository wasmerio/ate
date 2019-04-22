/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.common;

import com.google.api.client.repackaged.com.google.common.base.Objects;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.UUID;

/**
 * Static methods for safely comparing strings and other primative types for equality
 * @author John Sharratt (johnathan.sharratt@gmail.com)
 */
public class CompareSafe {
    
    public static boolean stringEqual(@Nullable String a, @Nullable String b) {
        if (a == null && b != null) return false;
        if (a != null && b == null) return false;
        if (a == null && b == null) return true;
        if (Objects.equal(a, b) == false) return false;
        return true;
    }
    
    public static boolean longEqual(@Nullable Long a, @Nullable Long b) {
        if (a == null && b != null) return false;
        if (a != null && b == null) return false;
        if (a == null && b == null) return true;
        if (Objects.equal(a, b) == false) return false;
        return true;
    }
    
    public static boolean integerEqual(@Nullable Integer a, @Nullable Integer b) {
        if (a == null && b != null) return false;
        if (a != null && b == null) return false;
        if (a == null && b == null) return true;
        if (Objects.equal(a, b) == false) return false;
        return true;
    }
    
    public static boolean uuidEqual(@Nullable UUID a, @Nullable UUID b) {
        if (a == null && b != null) return false;
        if (a != null && b == null) return false;
        if (a == null && b == null) return true;
        if (Objects.equal(a, b) == false) return false;
        return true;
    }
}
