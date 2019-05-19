package com.tokera.examples.dao;

import com.tokera.ate.annotations.PermitParentType;
import com.tokera.ate.dao.base.BaseDaoRights;
import com.tokera.ate.units.DaoId;
import com.tokera.examples.enumeration.BuiltInRole;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.UUID;

@PermitParentType(Account.class)
public class AccountRole extends BaseDaoRights {
    public UUID id;
    public String name;
    public UUID account;
    @Nullable
    public String description;
    public BuiltInRole builtInRole = BuiltInRole.OTHER;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public AccountRole() {
    }

    public AccountRole(Account acc, String name) {
        this.id = UUID.randomUUID();
        this.account = acc.id;
        this.name = name;
    }

    public @DaoId UUID getId() {
        return this.id;
    }

    public @Nullable @DaoId UUID getParentId() {
        return this.account;
    }
}
