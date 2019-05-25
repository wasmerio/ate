package com.tokera.examples.dao;

import com.tokera.ate.annotations.ImplicitAuthority;
import com.tokera.ate.annotations.PermitParentFree;
import com.tokera.ate.common.ImmutalizableArrayList;
import com.tokera.ate.dao.base.BaseDaoRoles;
import com.tokera.ate.units.DaoId;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.math.BigDecimal;
import java.util.UUID;

@PermitParentFree
public class Asset extends BaseDaoRoles {
    public UUID id;
    @ImplicitAuthority
    public String type;
    public BigDecimal value;
    public ImmutalizableArrayList<UUID> shares = new ImmutalizableArrayList<UUID>();

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public Asset() {
    }

    public Asset(String type, BigDecimal value) {
        this.id = UUID.randomUUID();
        this.type = type;
        this.value = value;
    }

    public @DaoId UUID getId() {
        return id;
    }

    public @Nullable @DaoId UUID getParentId() {
        return null;
    }
}
