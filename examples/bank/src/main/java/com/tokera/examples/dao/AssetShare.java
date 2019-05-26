package com.tokera.examples.dao;

import com.tokera.ate.annotations.PermitParentType;
import com.tokera.ate.common.ImmutalizableArrayList;
import com.tokera.ate.dao.base.BaseDaoRolesRights;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.DaoId;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import java.math.BigDecimal;
import java.util.UUID;

@Dependent
@PermitParentType({Asset.class, AssetShare.class})
public class AssetShare extends BaseDaoRolesRights {
    public UUID id;
    public UUID parent;
    public UUID asset;
    public BigDecimal shareAmount;
    public ImmutalizableArrayList<UUID> shares = new ImmutalizableArrayList<UUID>();

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public AssetShare() {
    }

    @Override
    public @Alias String getRightsAlias() {
        return "ownership:" + parent + ":" + shareAmount;
    }

    public AssetShare(Asset asset, BigDecimal shareAmount) {
        this.id = UUID.randomUUID();
        this.parent = asset.id;
        this.asset = asset.id;
        this.shareAmount = shareAmount;
    }

    public AssetShare(AssetShare assetShare, BigDecimal shareAmount) {
        this.id = UUID.randomUUID();
        this.parent = assetShare.id;
        this.asset = assetShare.asset;
        this.shareAmount = shareAmount;
    }

    public @DaoId UUID getId() {
        return this.id;
    }

    public @Nullable @DaoId UUID getParentId() {
        return parent;
    }
}
