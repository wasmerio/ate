package com.tokera.examples.dao;

import com.tokera.ate.annotations.ImplicitAuthority;
import com.tokera.ate.annotations.PermitParentFree;
import com.tokera.ate.common.UUIDTools;
import com.tokera.ate.dao.base.BaseDaoRights;
import com.tokera.ate.units.DaoId;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.UUID;

@PermitParentFree
public class Company extends BaseDaoRights {
    @ImplicitAuthority
    public String domain;
    public UUID companyAccount;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public Company() {
    }

    public Company(String domain, Account companyAccount) {
        this.domain = domain;
        this.companyAccount = companyAccount.id;
    }

    public @DaoId UUID getId() {
        return UUIDTools.generateUUID(domain);
    }

    public @Nullable @DaoId UUID getParentId() {
        return null;
    }
}
