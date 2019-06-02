/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.security.core;

import java.nio.ByteBuffer;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
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
    private final AtomicLong index;
    
    public PredictablyRandomFactory(String seed) {
        try {
            this.digest = MessageDigest.getInstance("SHA-512");
        } catch (NoSuchAlgorithmException ex) {
            throw new WebApplicationException(ex);
        }
        this.seed = seed;
        this.index = new AtomicLong();
    }
    
    @Override
    public IRandom getRandom() {
        return new DigestBasedRandomWrapper(this);
    }

    public class DigestBasedRandomWrapper implements IRandom {
        private final PredictablyRandomFactory factory;

        public DigestBasedRandomWrapper(PredictablyRandomFactory factory) {
            this.factory = factory;
        }

        @Override
        public Random get() {
            long index = factory.index.incrementAndGet();
            String entropy = index + ":" + factory.seed;
            byte[] digestBytes = factory.digest.digest(entropy.getBytes());
            byte[] seedBytes = Arrays.copyOfRange(digestBytes, 0, 8);

            ByteBuffer buffer = ByteBuffer.allocate(Long.BYTES);
            buffer.put(seedBytes);
            buffer.flip();
            long seedLong = buffer.getLong();

            return new Random(seedLong);
        }

        @Override
        public void nextBytes(byte[] bytes) {
            get().nextBytes(bytes);
        }

        @Override
        public int nextInt() {
            return get().nextInt();
        }

        @Override
        public int nextInt(int bound) {
            return get().nextInt(bound);
        }

        @Override
        public long nextLong() {
            return get().nextLong();
        }

        @Override
        public boolean nextBoolean() {
            return get().nextBoolean();
        }

        @Override
        public float nextFloat() {
            return get().nextFloat();
        }

        @Override
        public double nextDouble() {
            return get().nextDouble();
        }

        @Override
        public double nextGaussian() {
            return get().nextGaussian();
        }

        @Override
        public IntStream ints(long streamSize) {
            return get().ints(streamSize);
        }

        @Override
        public IntStream ints() {
            return get().ints();
        }

        @Override
        public IntStream ints(long streamSize, int randomNumberOrigin, int randomNumberBound) {
            return get().ints(streamSize, randomNumberOrigin, randomNumberBound);
        }

        @Override
        public IntStream ints(int randomNumberOrigin, int randomNumberBound) {
            return get().ints(randomNumberOrigin, randomNumberBound);
        }

        @Override
        public LongStream longs(long streamSize) {
            return get().longs(streamSize);
        }

        @Override
        public LongStream longs() {
            return get().longs();
        }

        @Override
        public LongStream longs(long streamSize, long randomNumberOrigin, long randomNumberBound) {
            return get().longs(streamSize, randomNumberOrigin, randomNumberBound);
        }

        @Override
        public LongStream longs(long randomNumberOrigin, long randomNumberBound) {
            return get().longs(randomNumberOrigin, randomNumberBound);
        }

        @Override
        public DoubleStream doubles(long streamSize) {
            return get().doubles(streamSize);
        }

        @Override
        public DoubleStream doubles() {
            return get().doubles();
        }

        @Override
        public DoubleStream doubles(long streamSize, double randomNumberOrigin, double randomNumberBound) {
            return get().doubles(streamSize, randomNumberOrigin, randomNumberBound);
        }

        @Override
        public DoubleStream doubles(double randomNumberOrigin, double randomNumberBound) {
            return get().doubles(randomNumberOrigin, randomNumberBound);
        }
    }
}
