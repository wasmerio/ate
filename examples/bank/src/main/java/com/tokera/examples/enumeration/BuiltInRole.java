package com.tokera.examples.enumeration;

import javax.enterprise.context.Dependent;

@Dependent
public enum BuiltInRole {

    OWNER("Owner", "Party is the legal owner"),
    ATTORNEY( "Attorney", "Party has power of attorney"),
    AUDITOR("Auditor", "Party has the right to audit"),
    OTHER("Other", "Party has a custom role");

    private final String name;
    private final String description;

    BuiltInRole(String name, String description) {
        this.name = name;
        this.description = description;
    }

    public String getName() {
        return name;
    }
}
