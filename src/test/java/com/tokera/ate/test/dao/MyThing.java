package com.tokera.ate.test.dao;

import com.tokera.ate.annotations.PermitParentType;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.units.DaoId;

import javax.persistence.Column;
import javax.persistence.Table;
import java.util.UUID;

@YamlTag("dao.mything")
@Table(name = "dao.mything")
@PermitParentType(MyAccount.class)
public class MyThing {
    @Column
    public @DaoId UUID accountId;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public MyThing() {
    }

    public MyThing(MyAccount acc) {
        this.accountId = acc.id;
    }
}
