package com.tokera.ate.test.dao;

import com.tokera.ate.annotations.PermitParentFree;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.common.ImmutalizableArrayList;
import com.tokera.ate.common.ImmutalizableTreeMap;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.TextDocument;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.persistence.Column;
import javax.persistence.Table;
import java.util.UUID;

@YamlTag("dao.mybaseaccount")
@Table(name = "mybaseaccount")
@PermitParentFree
public class MyBaseAccount {
    @Column
    public final UUID id = UUID.randomUUID();
    @Nullable
    @Column
    public @TextDocument String description = null;

    public MyBaseAccount() { }
}
