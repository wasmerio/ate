/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dao.base;

import com.tokera.ate.common.Immutalizable;
import com.tokera.ate.common.ImmutalizableHashSet;
import com.tokera.ate.dao.IRights;
import com.tokera.ate.dao.IRoles;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.units.Alias;

import java.util.Set;
import javax.persistence.Column;

/**
 * Represents the common fields and methods of all data objects that are stored in the ATE data-store
 * plus it holds holds access rights to different read and write roles throughout the data model.
 * plus a set of user-defined key-value parameters that can be associated with the data object
 * If a user is able to read this record then they are able to gain access to the things that it has access to
 */
public abstract class BaseDaoRolesRights extends BaseDaoRoles implements IRights, Immutalizable
{
    @Column
    public final ImmutalizableHashSet<MessagePrivateKeyDto> rightsRead = new ImmutalizableHashSet<>();
    @Column
    public final ImmutalizableHashSet<MessagePrivateKeyDto> rightsWrite = new ImmutalizableHashSet<>();
    
    @Override
    public Set<MessagePrivateKeyDto> getRightsRead() {
        return rightsRead;
    }

    @Override
    public Set<MessagePrivateKeyDto> getRightsWrite() {
        return rightsWrite;
    }

    @Override
    public @Alias String getRightsAlias() {
        return this.getId().toString();
    }

    @Override
    public void onAddRight(IRoles to) {
    }

    @Override
    public void onRemoveRight(IRoles from) {
    }

    @Override
    public void immutalize() {
        super.immutalize();
        this.rightsRead.immutalize();
        this.rightsWrite.immutalize();
    }
}
