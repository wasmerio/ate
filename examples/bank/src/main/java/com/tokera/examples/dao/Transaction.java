package com.tokera.examples.dao;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.dao.PUUID;

import javax.enterprise.context.Dependent;
import java.math.BigDecimal;
import java.util.UUID;

@Dependent
public class Transaction {
    @JsonProperty
    public UUID id;
    @JsonProperty
    public String description;
    @JsonProperty
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