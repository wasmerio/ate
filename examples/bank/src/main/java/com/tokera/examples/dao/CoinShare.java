package com.tokera.examples.dao;

import com.tokera.ate.annotations.PermitParentType;
import com.tokera.ate.common.ImmutalizableArrayList;
import com.tokera.ate.dao.base.BaseDaoRoles;
import com.tokera.ate.units.DaoId;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import java.math.BigDecimal;
import java.util.UUID;

@Dependent
@PermitParentType({Coin.class, CoinShare.class})
public class CoinShare extends BaseDaoRoles {
    public UUID id;
    public UUID parent;
    public UUID asset;
    public String type;
    public BigDecimal shareAmount;
    public ImmutalizableArrayList<UUID> shares = new ImmutalizableArrayList<UUID>();

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public CoinShare() {
    }

    public CoinShare(Coin coin, BigDecimal shareAmount) {
        this.id = UUID.randomUUID();
        this.parent = coin.id;
        this.asset = coin.id;
        this.type = coin.type;
        this.shareAmount = shareAmount;
    }

    public CoinShare(CoinShare coinShare, BigDecimal shareAmount) {
        this.id = UUID.randomUUID();
        this.parent = coinShare.id;
        this.asset = coinShare.asset;
        this.type = coinShare.type;
        this.shareAmount = shareAmount;
    }

    public @DaoId UUID getId() {
        return this.id;
    }

    public @Nullable @DaoId UUID getParentId() {
        return parent;
    }
}
