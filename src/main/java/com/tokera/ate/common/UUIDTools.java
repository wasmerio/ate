/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.common;

import com.tokera.ate.dao.ObjId;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.io.UnsupportedEncodingException;
import java.util.UUID;

/**
 * Class that provides helper functions for converting strings to and from UUID primative types
 */
public class UUIDTools {
    
    public static UUID convertUUID(String val)
    {
        return parseUUID(val);
    }

    public static UUID generateUUID(String val)
    {
        try {
            byte[] bytes = val.getBytes("UTF-8");
            return UUID.nameUUIDFromBytes(bytes);
        } catch (UnsupportedEncodingException ex) {
            throw new RuntimeException(ex);
        }
    }

    public static @Nullable UUID convertUUIDOrNull(@Nullable ObjId id)
    {
        if (id == null) return null;
        return new UUID(id.high(), id.low());
    }
    
    public static UUID convertUUID(ObjId id)
    {
        return new UUID(id.high(), id.low());
    }

    public static UUID parseUUID(String val)
    {
        val = val.trim();

        if (val.length() <= 0) {
            throw new IllegalArgumentException("Input string can not be empty.");
        }

        if ("null".equals(val) || "[null]".equals(val)) {
            throw new IllegalArgumentException("Input string would result in a null reference.");
        }

        if (val.startsWith(".")) {
            val = val.substring(1);
        }

        if (val.toLowerCase().startsWith("import://") == true) {
            val = val.substring("import://".length());
        }

        if (val.contains(".")) {
            val = val.substring(val.lastIndexOf(".") + 1);
        }

        return UUID.fromString(val);
    }

    public static @Nullable UUID parseUUIDorNull(@Nullable String _val)
    {
        String val = _val;
        if (val == null) return null;
        val = val.trim();
        
        if (val.length() <= 0)
            return null;
        
        if ("null".equals(val) ||
            "[null]".equals(val))
            return null;
        
        if (val.startsWith(".")) {
            val = val.substring(1);
        }
        
        if (val.toLowerCase().startsWith("import://") == true) {
            val = val.substring("import://".length());
        }

        if (val.contains(".")) {
            val = val.substring(val.lastIndexOf(".") + 1);
        }
        
        try
        {
            return UUID.fromString(val);
        }
        catch (IllegalArgumentException ex)
        {
            return null;
        }
    }
}
