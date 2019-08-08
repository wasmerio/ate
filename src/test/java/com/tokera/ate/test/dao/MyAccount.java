package com.tokera.ate.test.dao;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.PermitParentFree;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.common.ImmutalizableArrayList;
import com.tokera.ate.common.ImmutalizableTreeMap;
import com.tokera.ate.dao.CountLong;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.RangeLong;
import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.units.*;
import org.apache.commons.lang.math.LongRange;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
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
    @JsonProperty
    public final ImmutalizableArrayList<@DaoId UUID> things = new ImmutalizableArrayList<>();
    @JsonProperty
    public boolean isPublic = false;
    @JsonProperty
    public final ImmutalizableTreeMap<@Alias String, @DaoId UUID> textFiles = new ImmutalizableTreeMap<>();
    @JsonProperty
    public BigInteger num1 = BigInteger.ZERO;
    @JsonProperty
    public BigDecimal num2 = BigDecimal.ZERO;
    @JsonProperty
    public @Nullable @Hash String passwordHash;
    @JsonProperty
    @NotNull
    @Size(min=1, max=512)
    @Pattern(regexp="[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?", message="Invalid email")//if the field contains email address consider using this annotation to enforce field validation
    public @EmailAddress String email;
    @JsonProperty
    public @Nullable UUID idNullTest = null;
    @JsonProperty
    public PUUID pid = new PUUID("data1234", 1, UUID.randomUUID(), DataPartitionType.Dao);
    @JsonProperty
    public RangeLong range = new RangeLong(1, 10);
    @JsonProperty
    public CountLong counter = new CountLong(0L);

    public MyAccount() {
        this.email = "test@test.org";
    }

    public MyAccount(@EmailAddress String email, @Hash String passwordHash)
    {
        this.email = email;
        this.passwordHash = passwordHash;
    }
}
