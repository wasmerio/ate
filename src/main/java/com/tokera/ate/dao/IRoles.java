/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dao;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.Hash;
import com.tokera.ate.units.Secret;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.Map;

/**
 * Represents what who is allowed to access this data entity and its children
 * through the maintenance of roles
 */
public interface IRoles
{
    /**
     * @return Returns a list of all the public keys of those who have read
     * access to the objects held by this data entity and the children attached
     * to it
     */
    @JsonIgnore
    Map<@Alias String, @Hash String> getTrustAllowRead();

    /**
     * @return Returns a list of all the public keys of those who have write
     * access to the objects held by this data entity and the children attached
     * to it
     */
    @JsonIgnore
    Map<@Alias String, @Hash String> getTrustAllowWrite();
    
    /**
     * @return True if the chain of trust should inherit write permissions from its parent
     */
    @JsonIgnore
    boolean getTrustInheritWrite();
    
    /**
     * @return True if the chain of trust should inherit read permission from its parent
     */
    @JsonIgnore
    boolean getTrustInheritRead();
}
