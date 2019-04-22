/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.security.core;

import java.security.SecureRandom;
import java.util.Random;

/**
 * Random number generator that is unpredictable, this uses the SecureRandom number generator underneigh.
 */
public class UnPredictablyRandom implements IRandom {
    
    @Override
    public Random getRandom() {
        return new SecureRandom();
    }
}