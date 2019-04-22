/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.events;

import com.tokera.ate.dto.TokenDto;

/**
 * Event thats triggered whenever a new token is encountered
 */
public class TokenDiscoveryEvent {
    
    private TokenDto token;

    public TokenDiscoveryEvent(TokenDto token) {
        this.token = token;
    }

    public TokenDto getToken() {
        return token;
    }

    public void setToken(TokenDto token) {
        this.token = token;
    }
}
