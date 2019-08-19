package com.tokera.examples.dao;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.PermitParentType;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.units.DaoId;
import com.tokera.examples.common.CoinHelper;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import java.math.BigDecimal;
import java.util.*;
import java.util.stream.Collectors;

@Dependent
@PermitParentType(MonthlyActivity.class)
public class TransactionDetails extends BaseDao {
    @JsonProperty
    public UUID id;
    @JsonProperty
    public UUID monthlyActivity;
    @JsonProperty
    public BigDecimal amount;
    @JsonProperty
    public ArrayList<PUUID> shares = new ArrayList<PUUID>();
    @JsonProperty
    public Date when;
    @JsonProperty
    public String description;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public TransactionDetails() {
    }

    public TransactionDetails(MonthlyActivity monthly, Iterable<CoinShare> shares, String description) {
        this.id = UUID.randomUUID();
        this.monthlyActivity = monthly.id;
        this.when = new Date();
        this.amount = BigDecimal.ZERO;

        CoinHelper helper = new CoinHelper();
        for (CoinShare share : shares) {
            this.shares.add(share.addressableId());
            this.amount = this.amount.add(helper.valueOfShare(share, false));
        }
        this.description = description;
    }

    public @DaoId UUID getId() {
        return this.id;
    }

    public @Nullable @DaoId UUID getParentId() {
        return this.monthlyActivity;
    }
}
