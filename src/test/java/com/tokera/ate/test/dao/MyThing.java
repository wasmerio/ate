package com.tokera.ate.test.dao;

import com.tokera.ate.annotations.PermitParentType;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.units.DaoId;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import javax.persistence.Column;
import javax.persistence.Table;
import java.util.UUID;

@Dependent
@YamlTag("dao.mything")
@Table(name = "dao.mything")
@PermitParentType(MyAccount.class)
public class MyThing extends BaseDao {
    @Column
    public @DaoId UUID id = UUID.randomUUID();
    @Column
    public @DaoId UUID accountId;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public MyThing() {
    }

    public MyThing(MyAccount acc) {
        this.accountId = acc.id;
    }

    @Override
    public @DaoId UUID getId() {
        return id;
    }

    @Override
    public @Nullable @DaoId UUID getParentId() {
        return null;
    }
}
