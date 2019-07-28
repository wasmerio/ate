/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dao.base;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.common.Immutalizable;
import com.tokera.ate.common.ImmutalizableTreeMap;
import com.tokera.ate.dao.IRoles;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.Hash;
import com.tokera.ate.units.Secret;
import org.checkerframework.checker.nullness.qual.NonNull;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.Map;
/**
 * Represents what who is allowed to access this data entity and its children
 * through the maintenance of roles
 */
public abstract class BaseDaoParamsRoles extends BaseDaoParams implements IRoles, Immutalizable {

    @JsonProperty
    public final ImmutalizableTreeMap<@Alias String, @Hash String> trustAllowRead = new ImmutalizableTreeMap<>();
    @JsonProperty
    public final ImmutalizableTreeMap<@Alias String, @Hash String> trustAllowWrite = new ImmutalizableTreeMap<>();
    @JsonProperty
    public boolean trustInheritWrite = true;
    @JsonProperty
    public boolean trustInheritRead = true;

    /**
     * @return Returns a list of all the public toPutKeys of those who have read
     * access to the objects held by this data entity and the children attached
     * to it
     */
    @Override
    @JsonIgnore
    public @NonNull Map<@Alias String, @Hash String> getTrustAllowRead() {
        return trustAllowRead;
    }

    /**
     * @return Returns a list of all the public toPutKeys of those who have write
     * access to the objects held by this data entity and the children attached
     * to it
     */
    @Override
    @JsonIgnore
    public @NonNull Map<@Alias String, @Hash String> getTrustAllowWrite() {
        return trustAllowWrite;
    }

    /**
     * @return True if the chain of trust should inherit write permissions from its parent
     */
    @Override
    @JsonIgnore
    public boolean getTrustInheritWrite() {
        return trustInheritWrite;
    }

    /**
     * @return True if the chain of trust should inherit read permission from its parent
     */
    @Override
    @JsonIgnore
    public boolean getTrustInheritRead() {
        return trustInheritRead;
    }

    @Override
    public void immutalize() {
        super.immutalize();
        this.trustAllowRead.immutalize();
        this.trustAllowWrite.immutalize();
    }
}
