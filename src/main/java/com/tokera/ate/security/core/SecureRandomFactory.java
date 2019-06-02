/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.security.core;

import java.security.SecureRandom;
import java.util.Random;
import java.util.stream.DoubleStream;
import java.util.stream.IntStream;
import java.util.stream.LongStream;

/**
 * Random number generator that is unpredictable, this uses the SecureRandom number generator underneigh.
 */
public class SecureRandomFactory implements IRandomFactory {
    private final IRandom random = new SecureRandomWrapper(new SecureRandom());

    @Override
    public IRandom getRandom() {
        return random;
    }

    public class SecureRandomWrapper implements IRandom {
        private final SecureRandom random;

        public SecureRandomWrapper(SecureRandom random) {
            this.random = random;
        }

        @Override
        public Random get() {
            return random;
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