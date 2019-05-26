package com.tokera.examples.dao;

import com.tokera.ate.annotations.ImplicitAuthority;
import com.tokera.ate.annotations.ImplicitAuthorityField;
import com.tokera.ate.annotations.PermitParentFree;
import com.tokera.ate.common.UUIDTools;
import com.tokera.ate.dao.base.BaseDaoRolesRights;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.DaoId;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import java.util.UUID;

@Dependent
@PermitParentFree
public class Company extends BaseDaoRolesRights {
    @ImplicitAuthorityField
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

    @Alias
    public String getRightsAlias(){
        return domain;
    }
}
