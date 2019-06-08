package com.tokera.examples.dao;

import com.tokera.ate.dao.PUUID;

import java.math.BigDecimal;
import java.util.UUID;

public class Transaction {
    public UUID id;
    public String description;
    public BigDecimal amount;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public Transaction() {
    }

    public Transaction(TransactionDetails details) {
        this.id = details.id;
        this.amount = details.amount;
        this.description = details.description;
    }
}