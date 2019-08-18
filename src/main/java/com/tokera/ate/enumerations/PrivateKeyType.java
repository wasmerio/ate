package com.tokera.ate.enumerations;

import com.tokera.ate.annotations.YamlTag;

import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.Response;

public enum PrivateKeyType {
    read("r"),
    write("w");

    private String shortName;

    PrivateKeyType(String shortName) {
        this.shortName = shortName;
    }

    public static PrivateKeyType parse(String val) {
        for (PrivateKeyType e : PrivateKeyType.values()) {
            if (val.equalsIgnoreCase(e.shortName)) return e;
            if (val.equalsIgnoreCase(e.name())) return e;
        }
        throw new WebApplicationException("Unable to parse the string [" + val + "] into a private key type.", Response.Status.INTERNAL_SERVER_ERROR);
    }

    public String shortName() {
        return this.shortName;
    }
}
