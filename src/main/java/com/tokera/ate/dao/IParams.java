/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dao;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.PlainText;
import com.tokera.ate.units.Secret;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.Map;
import java.util.UUID;

/**
 * Allows the storage of plain text and encrypted secrets as key value pairs
 */
public interface IParams {

    @JsonIgnore
    @DaoId UUID getId();

    @JsonIgnore
    Map<@Alias String, @PlainText String> getParams();

    @JsonIgnore
    Map<@Alias String, @Secret String> getParamsEnc();

    @JsonIgnore
    boolean getShowParamsYml();

    @JsonIgnore
    void setShowParamsYml(boolean showParamsYml);

    @JsonIgnore
    boolean getHideParamsYml();

    @JsonIgnore
    void setHideParamsYml(boolean hideParamsYml);

    @JsonIgnore
    @Nullable @Secret String getParamsKey();

    @JsonIgnore
    void setParamsKey(@Nullable @Secret String paramsKey);
}
