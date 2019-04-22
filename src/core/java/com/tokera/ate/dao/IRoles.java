/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dao;

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
     * @return Returns a list of all the public toPutKeys of those who have read
     * access to the objects held by this data entity and the children attached
     * to it
     */
    Map<@Alias String, @Hash String> getTrustAllowRead();

    /**
     * @return Returns a list of all the public toPutKeys of those who have write
     * access to the objects held by this data entity and the children attached
     * to it
     */
    Map<@Alias String, @Hash String> getTrustAllowWrite();
    
    /**
     * @return The encryption key used to encrypt this data entity only for those that have read access
     */
    @Nullable @Secret String getEncryptKey();

    /**
     * @param encryptKey Sets the encryption key so that only owners with read access will be able to see the actual data
     */
    void setEncryptKey(@Secret String encryptKey);
    
    /**
     * @return True if the chain of trust should inherit write permissions from its parent
     */
    boolean getTrustInheritWrite();

    /**
     * @param val Set to true if the object should inherit write rights from its parent
     */
    void setTrustInheritWrite(boolean val);
    
    /**
     * @return True if the chain of trust should inherit read permission from its parent
     */
    boolean getTrustInheritRead();

    /**
     * @param val Set to true if the object should inherit read rights from its parent
     */
    void setTrustInheritRead(boolean val);
}
