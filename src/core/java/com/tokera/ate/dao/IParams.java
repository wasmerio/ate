/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dao;

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

    @DaoId UUID getId();

    Map<@Alias String, @PlainText String> getParams();

    Map<@Alias String, @Secret String> getParamsEnc();

    boolean getShowParamsYml();

    void setShowParamsYml(boolean showParamsYml);

    boolean getHideParamsYml();

    void setHideParamsYml(boolean hideParamsYml);

    @Nullable @Secret String getParamsKey();

    void setParamsKey(@Secret String paramsKey);
}
