package com.tokera.ate.test.dao;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.PermitParentFree;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.dao.base.BaseDaoParamsRoles;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.TextDocument;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import java.util.UUID;

@Dependent
@YamlTag("dao.mybaseaccount")
@PermitParentFree
public class MyBaseAccount extends BaseDaoParamsRoles {
    @JsonProperty
    public UUID id = UUID.randomUUID();
    @JsonProperty
    public @Nullable @TextDocument String description = null;
    @JsonProperty
    public float f1 = 0.0f;
    @JsonProperty
    public double d1 = 0.0;

    public MyBaseAccount() { }

    @Override
    @JsonIgnore
    public @DaoId UUID getId() {
        return this.id;
    }

    @Override
    @JsonIgnore
    public @Nullable @DaoId UUID getParentId() {
        return null;
    }
}
