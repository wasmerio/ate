/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.scope;

import com.tokera.server.api.dto.TokenDto;

/**
 *
 * @author John
 */
public class TokenDiscovery {
    
    private TokenDto token;
    private boolean isValidated = false;

    public TokenDiscovery(TokenDto token) {
        this.token = token;
    }

    /**
     * @return the token
     */
    public TokenDto getToken() {
        return token;
    }

    /**
     * @param token the token to set
     */
    public void setToken(TokenDto token) {
        this.token = token;
    }

    /**
     * @return the isValidated
     */
    public boolean isValidated() {
        return this.isValidated;
    }

    /**
     * @param isValidated the isValidated to set
     */
    public void setIsValidated(boolean isValidated) {
        this.isValidated = isValidated;
    }
}
