package com.tokera.ate.dao.enumerations;

import java.util.EnumSet;
import java.util.HashMap;
import java.util.Map;

public enum KeyType {
    unknown(0),
    ntru(1),
    ntru_sign(2),
    qtesla(3),
    newhope(4),
    xmss(5),
    xmssmt(6),
    rainbow(7);

    private static final Map<Integer,KeyType> lookup
            = new HashMap<Integer,KeyType>();

    static {
        for(KeyType s : EnumSet.allOf(KeyType.class))
            lookup.put(s.getCode(), s);
    }

    private int code;

    KeyType(int code) {
        this.code =code;
    }

    public int getCode() { return this.code; }

    public static KeyType get(int code) {
        return lookup.get(code);
    }
}
