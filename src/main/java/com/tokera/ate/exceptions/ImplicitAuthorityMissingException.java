package com.tokera.ate.exceptions;

/**
 * Exception thats thrown when key generation fails
 */
public class ImplicitAuthorityMissingException extends RuntimeException {

    public ImplicitAuthorityMissingException() {
    }

    public ImplicitAuthorityMissingException(String var1) {
        super(var1);
    }

    public ImplicitAuthorityMissingException(String var1, Throwable var2) {
        super(var1, var2);
    }

    public ImplicitAuthorityMissingException(Throwable var1) {
        super(var1);
    }
}
