package com.tokera.ate.test.dao;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.PermitParentFree;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.common.ImmutalizableArrayList;
import com.tokera.ate.common.ImmutalizableTreeMap;
import com.tokera.ate.units.*;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import javax.persistence.Column;
import javax.persistence.Table;
import javax.validation.constraints.NotNull;
import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.UUID;

@Dependent
@YamlTag("dao.myaccount")
@PermitParentFree
public class MyAccount extends MyBaseAccount {
    @Column
    public final ImmutalizableArrayList<@DaoId UUID> things = new ImmutalizableArrayList<>();
    @Column
    public boolean isPublic = false;
    @Column
    public final ImmutalizableTreeMap<@Alias String, @DaoId UUID> textFiles = new ImmutalizableTreeMap<>();
    @Column
    public BigInteger num1 = BigInteger.ZERO;
    @Column
    public BigDecimal num2 = BigDecimal.ZERO;
    @Column
    public @Nullable @Hash String passwordHash;
    @Column
    @NotNull
    @Size(min=1, max=512)
    @Pattern(regexp="[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?", message="Invalid email")//if the field contains email address consider using this annotation to enforce field validation
    private @EmailAddress String email;

    public MyAccount() {
        this.email = "test@test.org";
    }

    public MyAccount(@EmailAddress String email, @Hash String passwordHash)
    {
        this.email = email;
        this.passwordHash = passwordHash;
    }
}
