package com.tokera.examples.dto;

import com.tokera.ate.dto.TokenDto;

import java.util.UUID;

public class RegistrationResponse {
    private String token;
    private UUID id;
    private UUID accountId;

    public RegistrationResponse() {
    }

    public RegistrationResponse(UUID id, UUID accountId, TokenDto token) {
        this.id = id;
        this.accountId = accountId;
        this.token = token.getXmlToken();
    }

    public String getToken() {
        return token;
    }

    public void setToken(String token) {
        this.token = token;
    }

    public UUID getId() {
        return id;
    }

    public void setId(UUID id) {
        this.id = id;
    }

    public UUID getAccountId() {
        return accountId;
    }

    public void setAccountId(UUID accountId) {
        this.accountId = accountId;
    }
}
