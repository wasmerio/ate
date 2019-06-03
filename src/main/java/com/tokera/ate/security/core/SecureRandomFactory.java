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
    private final SecureRandom random = new SecureRandom();

    public SecureRandomFactory() {
    }

    @Override
    public SecureRandom getRandom() {
        return random;
    }
}