package com.tokera.ate.security.core;

import java.util.Random;
import java.util.stream.DoubleStream;
import java.util.stream.IntStream;
import java.util.stream.LongStream;

public interface IRandom {

    Random get();

    void nextBytes(byte[] bytes);

    int nextInt();

    int nextInt(int bound);

    long nextLong();

    boolean nextBoolean();

    float nextFloat();

    double nextDouble();

    double nextGaussian();

    IntStream ints(long streamSize);

    IntStream ints();

    IntStream ints(long streamSize, int randomNumberOrigin, int randomNumberBound);

    IntStream ints(int randomNumberOrigin, int randomNumberBound);

    LongStream longs(long streamSize);

    LongStream longs();

    LongStream longs(long streamSize, long randomNumberOrigin, long randomNumberBound);

    LongStream longs(long randomNumberOrigin, long randomNumberBound);

    DoubleStream doubles(long streamSize);

    DoubleStream doubles();

    DoubleStream doubles(long streamSize, double randomNumberOrigin, double randomNumberBound);

    DoubleStream doubles(double randomNumberOrigin, double randomNumberBound);
}
