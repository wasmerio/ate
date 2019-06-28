package com.tokera.ate.dao.enumerations;

public enum PermissionPhase {
    BeforeMerge("Permission rights for this object before its saved into the chain-of-trust."),
    AfterMerge("Permission rights for this object after its saved into the chain-of-trust."),
    DynamicStaging("Will be determined based on if the object is yet in the staging area or not"),
    DynamicChain("Will be determined based on what if the object has been pushed to the chain-of-trust yet");

    private final String description;

    PermissionPhase(String description) {
        this.description = description;
    }

    public String getDescription() {
        return this.description;
    }
}
