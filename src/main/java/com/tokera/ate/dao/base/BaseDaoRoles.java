/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dao.base;

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
public abstract class BaseDaoRoles extends BaseDaoParams implements IRoles, Immutalizable {

    @JsonProperty
    public final ImmutalizableTreeMap<@Alias String, @Hash String> trustAllowRead = new ImmutalizableTreeMap<>();
    @JsonProperty
    public final ImmutalizableTreeMap<@Alias String, @Hash String> trustAllowWrite = new ImmutalizableTreeMap<>();
    @JsonProperty
    public @Nullable @Secret String encryptKey = null;
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
    public @NonNull Map<@Alias String, @Hash String> getTrustAllowRead() {
        return trustAllowRead;
    }

    /**
     * @return Returns a list of all the public toPutKeys of those who have write
     * access to the objects held by this data entity and the children attached
     * to it
     */
    @Override
    public @NonNull Map<@Alias String, @Hash String> getTrustAllowWrite() {
        return trustAllowWrite;
    }

    /**
     * @return The encryption key used to encrypt this data entity only for those that have read access
     */
    @Override
    public @Nullable @Secret String getEncryptKey() {
        return encryptKey;
    }

    /**
     * @param encryptKey Sets the encryption key so that only owners with read access will be able to see the actual data
     */
    @Override
    public void setEncryptKey(@Nullable @Secret String encryptKey) {
        assert this._immutable == false;
        this.encryptKey = encryptKey;
    }

    /**
     * @return True if the chain of trust should inherit write permissions from its parent
     */
    @Override
    public boolean getTrustInheritWrite() {
        return trustInheritWrite;
    }

    /**
     * @param val Set to true if the object should inherit write rights from its parent
     */
    @Override
    public void setTrustInheritWrite(boolean val) {
        assert this._immutable == false;
        this.trustInheritWrite = val;
    }

    /**
     * @return True if the chain of trust should inherit read permission from its parent
     */
    @Override
    public boolean getTrustInheritRead() {
        return trustInheritRead;
    }

    /**
     * @param val Set to true if the object should inherit read rights from its parent
     */
    @Override
    public void setTrustInheritRead(boolean val) {
        assert this._immutable == false;
        this.trustInheritRead = val;
    }

    @Override
    public void immutalize() {
        super.immutalize();
        this.trustAllowRead.immutalize();
        this.trustAllowWrite.immutalize();
    }
}
