package com.tokera.examples.dto;

import com.tokera.ate.dto.msg.MessagePrivateKeyDto;

import javax.enterprise.context.Dependent;
import java.util.ArrayList;

@Dependent
public class RootLoginRequest {
    private String username = "root";
    private ArrayList<MessagePrivateKeyDto> readRights = new ArrayList<MessagePrivateKeyDto>();
    private ArrayList<MessagePrivateKeyDto> writeRights = new ArrayList<MessagePrivateKeyDto>();

    public String getUsername() {
        return username;
    }

    public void setUsername(String username) {
        this.username = username;
    }

    public ArrayList<MessagePrivateKeyDto> getReadRights() {
        return readRights;
    }

    public void setReadRights(ArrayList<MessagePrivateKeyDto> readRights) {
        this.readRights = readRights;
    }

    public ArrayList<MessagePrivateKeyDto> getWriteRights() {
        return writeRights;
    }

    public void setWriteRights(ArrayList<MessagePrivateKeyDto> writeRights) {
        this.writeRights = writeRights;
    }
}
