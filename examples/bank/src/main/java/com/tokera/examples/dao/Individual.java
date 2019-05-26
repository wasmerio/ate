package com.tokera.examples.dao;

import com.tokera.ate.annotations.ClaimableAuthority;
import com.tokera.ate.annotations.PermitParentFree;
import com.tokera.ate.common.UUIDTools;
import com.tokera.ate.dao.base.BaseDaoRights;
import com.tokera.ate.units.*;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;
import java.util.UUID;

@ClaimableAuthority
@PermitParentFree
public class Individual extends BaseDaoRights {
    @Size(max = 128)
    @Pattern(regexp="(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|\"(?:[\\x01-\\x08\\x0b\\x0c\\x0e-\\x1f\\x21\\x23-\\x5b\\x5d-\\x7f]|\\\\[\\x01-\\x09\\x0b\\x0c\\x0e-\\x7f])*\")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\\[(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?|[a-z0-9-]*[a-z0-9]:(?:[\\x01-\\x08\\x0b\\x0c\\x0e-\\x1f\\x21-\\x5a\\x53-\\x7f]|\\\\[\\x01-\\x09\\x0b\\x0c\\x0e-\\x7f])+)\\])")
    public String email;
    public UUID personalAccount;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public Individual() {
    }

    public Individual(String email, Account personalAccount) {
        this.email = email;
        this.personalAccount = personalAccount.id;
    }

    public @DaoId UUID getId() {
        return UUIDTools.generateUUID(this.email);
    }

    public @Nullable @DaoId UUID getParentId() {
        return null;
    }
}
