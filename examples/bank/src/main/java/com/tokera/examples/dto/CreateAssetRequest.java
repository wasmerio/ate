package com.tokera.examples.dto;

import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;

import java.math.BigDecimal;

public class CreateAssetRequest {
    public String type;
    public BigDecimal value;
    public PrivateKeyWithSeedDto ownershipKey;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public CreateAssetRequest() {
    }

    public CreateAssetRequest(String type, BigDecimal value, PrivateKeyWithSeedDto ownershipKey) {
        this.type = type;
        this.value = value;
        this.ownershipKey = ownershipKey;
    }
}
