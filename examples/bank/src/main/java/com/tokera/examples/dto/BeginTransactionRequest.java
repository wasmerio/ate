package com.tokera.examples.dto;

import java.math.BigDecimal;

public class BeginTransactionRequest {
    public BigDecimal amount;
    public String assetType;

    public BeginTransactionRequest() {
    }

    public BeginTransactionRequest(BigDecimal amount, String assetType) {
        this.amount = amount;
        this.assetType = assetType;
    }
}