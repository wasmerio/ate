package com.tokera.examples.dao;

import org.checkerframework.checker.nullness.qual.Nullable;

import java.math.BigDecimal;
import java.util.UUID;

public class Transaction {
    public UUID id;
    public BigDecimal amount;
    @Nullable
    public String description;
}