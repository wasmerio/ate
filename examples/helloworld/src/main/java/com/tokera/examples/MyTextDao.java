package com.tokera.examples;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.PermitParentFree;
import com.tokera.ate.dao.base.BaseDaoRoles;
import com.tokera.ate.units.DaoId;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import java.util.UUID;

@Dependent
@PermitParentFree
public class MyTextDao extends BaseDaoRoles {
    @JsonProperty
    public UUID id;
    @JsonProperty
    public String text;

    public MyTextDao() {
        this.id = UUID.randomUUID();
    }

    @Override
    public @DaoId UUID getId() {
        return this.id;
    }

    @Override
    public @Nullable @DaoId UUID getParentId() {
        return null;
    }
}
