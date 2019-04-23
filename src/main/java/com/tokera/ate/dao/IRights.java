/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dao;

import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.DaoId;

import java.util.Set;
import java.util.UUID;

/**
 * Interface that provides access rights to different roles through the Tokera
 * ecosystem. If a user is able to read this record then they are able to
 * gain access to the things that it has access to
 */
public interface IRights
{
    @DaoId UUID getId();

    Set<MessagePrivateKeyDto> getRightsRead();

    Set<MessagePrivateKeyDto> getRightsWrite();
    
    @Alias String getRightsAlias();

    void onAddRight(IRoles to);

    void onRemoveRight(IRoles from);
}