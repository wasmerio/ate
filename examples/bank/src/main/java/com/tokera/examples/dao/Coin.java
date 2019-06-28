package com.tokera.examples.dao;

import com.tokera.ate.annotations.ImplicitAuthorityField;
import com.tokera.ate.annotations.PermitParentFree;
import com.tokera.ate.common.ImmutalizableArrayList;
import com.tokera.ate.dao.base.BaseDaoRoles;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.api.IPartitionKeyProvider;
import com.tokera.ate.units.DaoId;
import com.tokera.examples.common.CoinPartitionKey;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import java.math.BigDecimal;
import java.util.UUID;

@Dependent
@PermitParentFree
public class Coin extends BaseDaoRoles implements IPartitionKeyProvider {
    public UUID id;
    @ImplicitAuthorityField
    public String type;
    public BigDecimal value;
    public ImmutalizableArrayList<UUID> shares = new ImmutalizableArrayList<UUID>();

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public Coin() {
    }

    public Coin(String type, BigDecimal value) {
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

    @Override
    public IPartitionKey partitionKey(boolean shouldThrow) {
        return new CoinPartitionKey();
    }
}
