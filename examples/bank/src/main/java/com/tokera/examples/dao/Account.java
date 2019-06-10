package com.tokera.examples.dao;

import com.tokera.ate.annotations.PermitParentType;
import com.tokera.ate.common.ImmutalizableArrayList;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDaoRights;
import com.tokera.ate.units.DaoId;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import java.util.UUID;

@Dependent
@PermitParentType({Company.class, Individual.class})
public class Account extends BaseDaoRights {
    public UUID id;
    public String name;
    @Nullable
    public UUID company;
    @Nullable
    public UUID individual;
    public final List<UUID> monthlyActivities = new ImmutalizableArrayList<UUID>();
    public final Set<PUUID> coins = new HashSet<>();

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public Account() {
    }

    public Account(String name) {
        this.id = UUID.randomUUID();
        this.name = name;
    }

    public @DaoId UUID getId() {
        return this.id;
    }

    public @Nullable @DaoId UUID getParentId() {
        if (company != null) {
            return company;
        }
        if (individual != null) {
            return individual;
        }
        return null;
    }
}
