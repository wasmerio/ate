/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.security.core;

import java.nio.ByteBuffer;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.Arrays;
import java.util.Random;
import javax.ws.rs.WebApplicationException;

/**
 * Random number generator that uses a SHA-256 hash of a seed value to generate a series of deterministic numbers
 * that otherwise look random if different seeds were used.
 */
public class PredictablyRandom implements IRandom {
    
    private final MessageDigest digest;
    private final String seed;
    private final Random random;
    
    public PredictablyRandom(String seed) {
        try {
            this.digest = MessageDigest.getInstance("SHA-256");
        } catch (NoSuchAlgorithmException ex) {
            throw new WebApplicationException(ex);
        }
        this.seed = seed;
        this.random = PredictablyRandom.getRandom(0L, this.seed, this.digest);
    }
    
    @Override
    public Random getRandom() {
        return this.getRandom(this.random.nextLong());
    }

    private Random getRandom(Long pressed) {
        return PredictablyRandom.getRandom(pressed, this.seed, this.digest);
    }
    
    private static Random getRandom(Long preseed, String seed, MessageDigest digest) {
        String entropy = preseed + seed;
        byte[] digestBytes = digest.digest(entropy.getBytes());
        byte[] seedBytes = Arrays.copyOfRange(digestBytes, 0, 8);
        
        ByteBuffer buffer = ByteBuffer.allocate(Long.BYTES);
        buffer.put(seedBytes);
        buffer.flip();
        long seedLong = buffer.getLong();
        
        return new Random(seedLong);
    }
}
