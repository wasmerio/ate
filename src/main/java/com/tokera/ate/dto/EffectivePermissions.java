/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.IRights;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.security.EffectivePermissionBuilder;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;
import com.tokera.ate.units.Secret;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.validation.constraints.NotNull;
import java.util.ArrayList;
import java.util.List;
import java.util.Set;
import java.util.UUID;

/**
 * Contains all the read and write roles and permissions for a particular requestContext, The main use of this class is to
 * query the permissions of data objects contained within the data store system.
 */
@YamlTag("dto.effective.permissions")
public class EffectivePermissions
{
    @JsonProperty
    @NotNull
    public List<@Hash String> rolesRead;
    @JsonProperty
    @NotNull
    public List<@Hash String> rolesWrite;
    @JsonProperty
    @NotNull
    public List<@Hash String> anchorRolesRead;
    @JsonProperty
    @NotNull
    public List<@Hash String> anchorRolesWrite;
    
    public EffectivePermissions() {
        this.rolesRead = new ArrayList<>();
        this.rolesWrite = new ArrayList<>();
        this.anchorRolesRead = new ArrayList<>();
        this.anchorRolesWrite = new ArrayList<>();
    }
    
    public boolean canRead(IRights entity) {
        Set<MessagePrivateKeyDto> privateKeys = entity.getRightsRead();
        for (MessagePrivateKeyDto privateKey : privateKeys) {
            if (this.rolesRead.contains(privateKey.getPublicKeyHash())) {
                return true;
            }
        }
        return false;
    }
    
    public boolean canWrite(IRights entity) {
        Set<MessagePrivateKeyDto> privateKeys = entity.getRightsWrite();
        for (MessagePrivateKeyDto privateKey : privateKeys) {
            if (this.rolesWrite.contains(privateKey.getPublicKeyHash())) {
                return true;
            }
        }
        return false;
    }

    public void addWriteRole(MessagePublicKeyDto key) {
        @Hash String hash = key.getPublicKeyHash();
        if (rolesWrite.contains(hash) == false) {
            rolesWrite.add(hash);
        }
        if (anchorRolesWrite.contains(hash) == false) {
            anchorRolesWrite.add(hash);
        }
    }

    public void addReadRole(MessagePublicKeyDto key) {
        @Hash String hash = key.getPublicKeyHash();
        if (rolesRead.contains(hash) == false) {
            rolesRead.add(hash);
        }
        if (anchorRolesRead.contains(hash) == false) {
            anchorRolesRead.add(hash);
        }
    }
}
