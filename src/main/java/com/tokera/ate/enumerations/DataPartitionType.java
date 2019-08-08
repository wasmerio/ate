package com.tokera.ate.enumerations;

import javax.ws.rs.WebApplicationException;

public enum DataPartitionType {
    Dao("D", 1),
    Io("I", 2),
    Publish("P", 3);

    DataPartitionType(String shortHand, int code) {
        this.shortHand = shortHand;
        this.code = code;
    }

    private final String shortHand;
    private final int code;

    public String getShortHand() {
        return this.shortHand;
    }

    public int getCode() {
        return this.code;
    }

    public static DataPartitionType fromCode(int code) {
        for (DataPartitionType type : values()) {
            if (code == type.code) return type;
        }
        throw new WebApplicationException("Unknown partition type code [" + code + "]");
    }

    public static DataPartitionType parse(String val) {
        for (DataPartitionType type : values()) {
            if (type.name().equalsIgnoreCase(val)) return type;
            if (type.shortHand.equalsIgnoreCase(val)) return type;
        }
        throw new WebApplicationException("Unknown partition type code [" + val + "]");
    }
}
