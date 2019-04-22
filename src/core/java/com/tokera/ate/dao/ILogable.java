/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dao;

import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.LogText;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.UUID;

/**
 * Interface to a data object that provides basic logging functionality
 */
public interface ILogable {

    @DaoId UUID getId();
    
    @Nullable @LogText String getError();
    
    void setError(@Nullable @LogText String val);

    @Nullable @LogText String getLog();
    
    void setLog(@Nullable @LogText String val);
}
