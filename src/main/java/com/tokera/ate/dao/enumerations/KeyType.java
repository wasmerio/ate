package com.tokera.ate.dao.enumerations;

import com.tokera.ate.common.MapTools;

import java.util.EnumSet;
import java.util.HashMap;
import java.util.Map;

public enum KeyType {
    unknown(0, KeyUse.unknown),
    ntru(1, KeyUse.encrypt),
    ntru_sign(2, KeyUse.sign),
    qtesla(3, KeyUse.sign),
    newhope(4, KeyUse.encrypt),
    xmss(5, KeyUse.sign),
    xmssmt(6, KeyUse.sign),
    rainbow(7, KeyUse.sign);

    private static final Map<Integer,KeyType> lookup
            = new HashMap<Integer,KeyType>();

    static {
        for(KeyType s : EnumSet.allOf(KeyType.class))
            lookup.put(s.getCode(), s);
    }

    private int code;
    private KeyUse use;

    KeyType(int code, KeyUse use) {
        this.code = code;
        this.use = use;
    }

    public int getCode() { return this.code; }

    public KeyUse getUse() { return this.use; }

    public static KeyType get(int code) {
        KeyType ret = MapTools.getOrNull(lookup, code);
        if (ret == null) throw new RuntimeException("Failed to map the value(" + code + ") to a valid KeyType.");
        return ret;
    }
}
