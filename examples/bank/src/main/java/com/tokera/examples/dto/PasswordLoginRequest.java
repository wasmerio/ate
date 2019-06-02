package com.tokera.examples.dto;

import com.tokera.ate.delegates.AteDelegate;

import javax.enterprise.context.Dependent;

@Dependent
public class PasswordLoginRequest {
    private String username;
    private String passwordHash;

    public String getUsername() {
        return username;
    }

    public void setUsername(String username) {
        this.username = username;
    }

    public String getPasswordHash() {
        return passwordHash;
    }

    public void setPasswordHash(String passwordHash) {
        this.passwordHash = passwordHash;
    }

    public void setPassword(String password) {
        this.passwordHash = AteDelegate.get().encryptor.hashShaAndEncode(password);
    }
}
