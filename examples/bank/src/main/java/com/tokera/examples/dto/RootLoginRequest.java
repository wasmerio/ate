package com.tokera.examples.dto;

import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;

import javax.enterprise.context.Dependent;
import java.util.ArrayList;

@Dependent
public class RootLoginRequest {
    private String username = "root";
    private ArrayList<PrivateKeyWithSeedDto> readRights = new ArrayList<PrivateKeyWithSeedDto>();
    private ArrayList<PrivateKeyWithSeedDto> writeRights = new ArrayList<PrivateKeyWithSeedDto>();

    public String getUsername() {
        return username;
    }

    public void setUsername(String username) {
        this.username = username;
    }

    public ArrayList<PrivateKeyWithSeedDto> getReadRights() {
        return readRights;
    }

    public void setReadRights(ArrayList<PrivateKeyWithSeedDto> readRights) {
        this.readRights = readRights;
    }

    public ArrayList<PrivateKeyWithSeedDto> getWriteRights() {
        return writeRights;
    }

    public void setWriteRights(ArrayList<PrivateKeyWithSeedDto> writeRights) {
        this.writeRights = writeRights;
    }
}
