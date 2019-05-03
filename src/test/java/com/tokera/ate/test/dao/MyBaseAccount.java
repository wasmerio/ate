package com.tokera.ate.test.dao;

import com.tokera.ate.annotations.PermitParentFree;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.common.ImmutalizableArrayList;
import com.tokera.ate.common.ImmutalizableTreeMap;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.base.BaseDaoRights;
import com.tokera.ate.dao.base.BaseDaoRoles;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.TextDocument;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import javax.persistence.Column;
import javax.persistence.Table;
import java.util.UUID;

@Dependent
@YamlTag("dao.mybaseaccount")
@PermitParentFree
public class MyBaseAccount extends BaseDaoRoles {
    @Column
    public UUID id = UUID.randomUUID();
    @Column
    public @Nullable @TextDocument String description = null;
    @Column
    public float f1 = 0.0f;
    @Column
    public double d1 = 0.0;

    public MyBaseAccount() { }

    @Override
    public @DaoId UUID getId() {
        return this.id;
    }

    @Override
    public @Nullable @DaoId UUID getParentId() {
        return null;
    }
}
