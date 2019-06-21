package com.tokera.ate.dao.enumerations;

import com.tokera.ate.common.MapTools;

import java.util.EnumSet;
import java.util.HashMap;
import java.util.Map;

public enum KeyUse {
    unknown(0),
    encrypt(1),
    sign(2),;

    private static final Map<Integer, KeyUse> lookup
            = new HashMap<Integer, KeyUse>();

    static {
        for(KeyUse s : EnumSet.allOf(KeyUse.class))
            lookup.put(s.getCode(), s);
    }

    private int code;

    KeyUse(int code) {
        this.code = code;
    }

    public int getCode() { return this.code; }

    public static KeyUse get(int code) {
        KeyUse ret = MapTools.getOrNull(lookup, code);
        if (ret == null) throw new RuntimeException("Failed to map the value(" + code + ") to a valid KeyUseType.");
        return ret;
    }
}
