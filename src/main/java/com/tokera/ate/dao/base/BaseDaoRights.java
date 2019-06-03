/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dao.base;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.common.Immutalizable;
import com.tokera.ate.common.ImmutalizableHashSet;
import com.tokera.ate.dao.IRights;
import com.tokera.ate.dao.IRoles;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.units.Alias;

import java.util.Set;

/**
 * Represents the common fields and methods of all data objects that are stored in the ATE data-store
 * plus it holds holds access rights to different read and write roles throughout the data model.
 * If a user is able to read this record then they are able to gain access to the things that it has access to
 */
public abstract class BaseDaoRights extends BaseDao implements IRights, Immutalizable
{
    @JsonProperty
    private final ImmutalizableHashSet<MessagePrivateKeyDto> rightsRead = new ImmutalizableHashSet<>();
    @JsonProperty
    private final ImmutalizableHashSet<MessagePrivateKeyDto> rightsWrite = new ImmutalizableHashSet<>();
    @Override
    public Set<MessagePrivateKeyDto> getRightsRead() {
        return rightsRead;
    }
    @Override
    public Set<MessagePrivateKeyDto> getRightsWrite() {
        return rightsWrite;
    }

    @Override
    public void immutalize() {
        super.immutalize();
        this.rightsRead.immutalize();
        this.rightsWrite.immutalize();
    }

    @Override
    public @Alias String getRightsAlias() {
        return getClass().getSimpleName().toLowerCase() + ":" + this.getId();
    }

    // Override this method to hook into notifications when an access right is added to this data object
    @Override
    public void onAddRight(IRoles to) {

    }

    // Override this method to hook into notifications when an access right is remove this data object
    @Override
    public void onRemoveRight(IRoles from) {

    }
}
