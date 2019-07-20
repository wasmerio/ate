/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.security.core;

import java.nio.ByteBuffer;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.security.NoSuchProviderException;
import java.security.SecureRandom;
import java.util.Arrays;
import java.util.Random;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;
import java.util.stream.DoubleStream;
import java.util.stream.IntStream;
import java.util.stream.LongStream;
import javax.ws.rs.WebApplicationException;

/**
 * Random number generator that uses a SHA-256 hash of a seed value to generate a series of deterministic numbers
 * that otherwise look random if different seeds were used.
 */
public class PredictablyRandomFactory implements IRandomFactory {
    private final MessageDigest digest;
    private final String seed;
    private SecureRandom random;
    
    public PredictablyRandomFactory(String seed) {
        try {
            this.digest = MessageDigest.getInstance("SHA-512");
            this.seed = seed;

        } catch (NoSuchAlgorithmException ex) {
            throw new RuntimeException(ex);
        }

        reset();
    }
    
    @Override
    public SecureRandom getRandom() {
        return this.random;
    }

    @Override
    public boolean idempotent() { return true; }

    @Override
    public void reset() {
        try {
            byte[] digestBytes = this.digest.digest(seed.getBytes());
            this.random = SecureRandom.getInstance("SHA1PRNG", "SUN");
            this.random.setSeed(digestBytes);
        } catch (NoSuchAlgorithmException | NoSuchProviderException ex) {
            throw new RuntimeException(ex);
        }
    }
}
