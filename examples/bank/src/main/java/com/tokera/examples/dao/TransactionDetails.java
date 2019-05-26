package com.tokera.examples.dao;

import com.tokera.ate.annotations.PermitParentType;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.units.DaoId;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import java.math.BigDecimal;
import java.util.Date;
import java.util.UUID;

@Dependent
@PermitParentType(MonthlyActivity.class)
public class TransactionDetails extends BaseDao {
    public UUID id;
    public UUID monthlyActivity;
    public BigDecimal amount;
    public PUUID assetOwnership;
    public Date when;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public TransactionDetails() {
    }

    public TransactionDetails(MonthlyActivity monthly, AssetShare ownership) {
        this.id = UUID.randomUUID();
        this.monthlyActivity = monthly.id;
        this.when = new Date();
        this.assetOwnership = ownership.addressableId();
        this.amount = ownership.shareAmount;
    }

    public @DaoId UUID getId() {
        return this.id;
    }

    public @Nullable @DaoId UUID getParentId() {
        return this.monthlyActivity;
    }
}
