package com.tokera.examples.dto;

import com.tokera.ate.dao.PUUID;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.math.BigDecimal;

public class BeginTransactionRequest {
    public BigDecimal amount;
    public PUUID destinationAccount;
    public PUUID asset;
    @Nullable
    public String description;
    @Nullable
    public String details;
}