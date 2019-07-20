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
import com.tokera.ate.dao.IParams;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.PlainText;
import com.tokera.ate.units.Secret;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.Map;

/**
 * Represents the common fields and methods of all data objects that are stored in the ATE data-store
 * plus a set of user-defined key-value parameters that can be associated with the data object
 */
public abstract class BaseDaoParams extends BaseDao implements IParams, Immutalizable {

    @JsonProperty
    public final ImmutalizableTreeMap<@Alias String, @PlainText String> params = new ImmutalizableTreeMap<>();
    @JsonProperty
    public final ImmutalizableTreeMap<@Alias String, @Secret String> paramsEnc = new ImmutalizableTreeMap<>();
    @JsonProperty
    public boolean showParamsYml = false;
    @JsonProperty
    public boolean hideParamsYml = false;
    @JsonProperty
    public @MonotonicNonNull @Secret String paramsKey;

    /**
     * @return the params
     */
    @Override
    public Map<@Alias String, @PlainText String> getParams() {
        return params;
    }

    /**
     * @return the paramsEnc
     */
    @Override
    public Map<@Alias String, @Secret String> getParamsEnc() {
        return paramsEnc;
    }

    /**
     * @return the showParamsYml
     */
    @Override
    public boolean getShowParamsYml() {
        return showParamsYml;
    }

    /**
     * @param showParamsYml the showParamsYml to set
     */
    @Override
    public void setShowParamsYml(boolean showParamsYml) {
        assert this._immutable == false;
        this.showParamsYml = showParamsYml;
    }

    /**
     * @return the hideParamsYml
     */
    @Override
    public boolean getHideParamsYml() {
        return hideParamsYml;
    }

    /**
     * @param hideParamsYml the hideParamsYml to set
     */
    @Override
    public void setHideParamsYml(boolean hideParamsYml) {
        assert this._immutable == false;
        this.hideParamsYml = hideParamsYml;
    }

    /**
     * @return the paramsKey
     */
    @Override
    public @Nullable @Secret String getParamsKey() {
        return paramsKey;
    }

    /**
     * @param paramsKey the paramsKey to set
     */
    @Override
    public void setParamsKey(@Secret String paramsKey) {
        this.paramsKey = paramsKey;
    }

    @JsonIgnore
    @Override
    public void immutalize() {
        super.immutalize();
        this.params.immutalize();
        this.paramsEnc.immutalize();
    }
}
