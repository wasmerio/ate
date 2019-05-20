package com.tokera.examples.dto;

import java.math.BigDecimal;

public class CreateAssetRequest {
    public String type;
    public BigDecimal value;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public CreateAssetRequest() {
    }

    public CreateAssetRequest(String type, BigDecimal value) {
        this.type = type;
        this.value = value;
    }
}
