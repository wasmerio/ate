package com.tokera.examples;

import com.tokera.ate.annotations.PermitParentFree;
import com.tokera.ate.dao.base.BaseDaoRoles;
import com.tokera.ate.units.DaoId;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import javax.persistence.Column;
import java.util.UUID;

@Dependent
@PermitParentFree
public class MyTextDao extends BaseDaoRoles {
    @Column
    public UUID id;
    @Column
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
