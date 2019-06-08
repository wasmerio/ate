package com.tokera.examples.dto;

public class RedeemAssetRequest {
    public TransactionToken transactionToken;
    public String validateType;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public RedeemAssetRequest() {
    }

    public RedeemAssetRequest(TransactionToken transactionToken, String validateType) {
        this.transactionToken = transactionToken;
        this.validateType = validateType;
    }
}
