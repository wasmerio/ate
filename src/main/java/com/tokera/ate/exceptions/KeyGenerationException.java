package com.tokera.ate.exceptions;

/**
 * Exception thats thrown when key generation fails
 */
public class KeyGenerationException extends RuntimeException {

    public KeyGenerationException() {
    }

    public KeyGenerationException(String var1) {
        super(var1);
    }

    public KeyGenerationException(String var1, Throwable var2) {
        super(var1, var2);
    }

    public KeyGenerationException(Throwable var1) {
        super(var1);
    }
}
