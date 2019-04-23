/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.delegates;

import com.tokera.ate.scopes.ResourceScoped;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * Used to count the number of times a particular method is invoked
 */
@ResourceScoped
public class ResourceStatsDelegate {

    private final AtomicInteger count = new AtomicInteger();

    public int get() {
        return count.get();
    }

    public int add() {
        return count.incrementAndGet();
    }

    public boolean compareAndSet(int expect, int update) {
        return count.compareAndSet(expect, update);
    }

    public int remove() {
        return count.decrementAndGet();
    }
}
