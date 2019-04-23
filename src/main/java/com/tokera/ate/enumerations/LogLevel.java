package com.tokera.ate.enumerations;

import com.tokera.ate.annotations.YamlTag;

/**
 * @author jonhanlee
 */
@YamlTag("enum.log.level")
public enum LogLevel {

    UNKNOWN(0),
    TRACE(1),
    DEBUG(2),
    INFO(3),
    WARNING(4),
    ERROR(5);

    private final int value;

    LogLevel(int value) {
        this.value = value;
    }

    public int getValue() {
        return value;
    }
}
