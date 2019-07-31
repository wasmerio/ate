package com.tokera.ate.exceptions;

import javax.ws.rs.WebApplicationException;

/**
 * Exception thats thrown when a transaction is aborted due to another error
 */
public class TransactionAbortedException extends RuntimeException {

    public TransactionAbortedException() {
    }

    public TransactionAbortedException(String var1) {
        super(var1);
    }

    public TransactionAbortedException(String var1, Throwable var2) {
        super(var1, var2);
    }

    public TransactionAbortedException(Throwable var1) {
        super("Transaction aborted: " + var1.getMessage(), var1);
    }
}
