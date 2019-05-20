package com.tokera.examples.dao;

import com.tokera.ate.annotations.PermitParentType;
import com.tokera.ate.common.ImmutalizableArrayList;
import com.tokera.ate.dao.base.BaseDaoRolesRights;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.DaoId;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.math.BigDecimal;
import java.util.UUID;

@PermitParentType({Asset.class, Share.class})
public class Share extends BaseDaoRolesRights {
    public UUID id;
    public UUID parent;
    public BigDecimal shareAmount;
    public ImmutalizableArrayList<UUID> shares = new ImmutalizableArrayList<UUID>();

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public Share() {
    }

    public @Alias String getRightsAlias() {
        return "ownership:" + parent + ":" + shareAmount;
    }

    public Share(Asset asset, BigDecimal shareAmount) {
        this.id = UUID.randomUUID();
        this.parent = asset.id;
        this.shareAmount = shareAmount;
    }

    public Share(Share share, BigDecimal shareAmount) {
        this.id = UUID.randomUUID();
        this.parent = share.id;
        this.shareAmount = shareAmount;
    }

    public @DaoId UUID getId() {
        return this.id;
    }

    public @Nullable @DaoId UUID getParentId() {
        return parent;
    }
}
