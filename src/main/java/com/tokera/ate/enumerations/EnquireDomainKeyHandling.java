package com.tokera.ate.enumerations;

public enum EnquireDomainKeyHandling {
    SilentIgnore(false),
    LogOnNull(false),
    ThrowOnError(true),
    ThrowOnNull(true);

    private boolean throwOnError;

    EnquireDomainKeyHandling(boolean throwOnError) {
        this.throwOnError = throwOnError;
    }

    public boolean shouldThrowOnError() {
        return this.throwOnError;
    }
}
