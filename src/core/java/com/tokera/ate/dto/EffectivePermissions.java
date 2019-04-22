/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.google.gson.annotations.Expose;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.IRights;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.units.Hash;
import com.tokera.ate.units.Secret;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.validation.constraints.NotNull;
import java.util.ArrayList;
import java.util.List;
import java.util.Set;

/**
 * Contains all the read and write roles and permissions for a particular requestContext, The main use of this class is to
 * query the permissions of data objects contained within the data store system.
 */
@YamlTag("dto.effective.permissions")
public class EffectivePermissions
{
    @Expose
    @JsonProperty
    @Nullable
    @Secret
    public String encryptKeyHash;
    @Expose
    @JsonProperty
    @NotNull
    public List<@Hash String> rolesRead;
    @Expose
    @JsonProperty
    @NotNull
    public List<@Hash String> rolesWrite;
    @Expose
    @JsonProperty
    @NotNull
    public List<@Hash String> anchorRolesRead;
    @Expose
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

    public void updateEncryptKeyFromObjIfNull(BaseDao obj) {
        if (this.encryptKeyHash == null) {
            AteDelegate d = AteDelegate.get();
            String encryptKey64 = d.daoHelper.getEncryptKey(obj, false, false);
            if (encryptKey64 != null) {
                byte[] encryptKey = Base64.decodeBase64(encryptKey64);
                this.encryptKeyHash = d.encryptor.hashShaAndEncode(encryptKey);
            }
        }
    }
}
