package com.tokera.examples.dao;

import com.tokera.ate.annotations.PermitParentType;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDaoRights;
import com.tokera.ate.units.DaoId;
import com.tokera.examples.enumeration.AssetType;
import com.tokera.examples.enumeration.CurrencyCode;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.math.BigDecimal;
import java.util.Date;
import java.util.UUID;

@PermitParentType(MonthlyActivity.class)
public class TransactionDetails extends BaseDaoRights {
    public UUID id;
    public UUID monthlyActivity;
    public BigDecimal amount;
    public PUUID assetOwnership;
    public Date when;
    @Nullable
    public String description;
    @Nullable
    public String details;
    public AssetType type;
    public CurrencyCode currencyCode = CurrencyCode.NON;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public TransactionDetails() {
    }

    public TransactionDetails(MonthlyActivity monthly, Asset asset, Share ownership) {
        this.id = UUID.randomUUID();
        this.monthlyActivity = monthly.id;
        this.when = new Date();
        this.assetOwnership = ownership.addressableId();
        this.type = asset.type;
        this.currencyCode = asset.currencyCode;
        this.amount = ownership.value;
        this.description = asset.description;
    }

    public @DaoId UUID getId() {
        return this.id;
    }

    public @Nullable @DaoId UUID getParentId() {
        return this.monthlyActivity;
    }
}
