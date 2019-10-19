package com.tokera.ate.io.core;

import com.google.common.collect.ImmutableSet;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.dao.TopicAndPartition;
import com.tokera.ate.delegates.AteDelegate;

import javax.inject.Inject;
import java.util.HashSet;
import java.util.Set;
import java.util.concurrent.atomic.AtomicBoolean;

public abstract class DataPartitionDaemon implements Runnable {
    protected AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    protected LoggerHook LOG;

    private Thread thread;
    private Object threadLock = new Object();

    private Set<TopicAndPartition> partitions = new HashSet<>();
    private Object partitionsLock = new Object();

    private volatile int shouldRun = 1;
    protected AtomicBoolean isRunning = new AtomicBoolean(false);

    public void addPartition(TopicAndPartition partition) {
        synchronized (partitionsLock) {
            if (partitions.add(partition)) {
                start();
            }
        }
    }

    public void removePartition(TopicAndPartition partition) {
        synchronized (partitionsLock) {
            if (partitions.remove(partition)) {
                if (partitions.isEmpty()) {
                    //stop();
                } else {
                    start();
                }
            }
        }
    }

    public Set<TopicAndPartition> listPartitions() {
        synchronized (partitionsLock) {
            return ImmutableSet.copyOf(partitions);
        }
    }

    protected void start() {
        synchronized (threadLock) {
            shouldRun = 1;
            if (isRunning.compareAndSet(false, true)) {
                this.thread = new Thread(this);
                this.thread.setDaemon(true);
                this.thread.start();
            }
        }
    }

    protected void stop() {
        synchronized (threadLock) {
            shouldRun = 0;
            if (this.thread != null) {
                this.thread.interrupt();
                try {
                    this.thread.join();
                } catch (InterruptedException e) {
                    LOG.warn(e);
                }
            }
        }
    }

    @Override
    public void run() {
        int exponential_backoff = 5;
        try {
            for (; shouldRun == 1; ) {
                try {
                    work();

                    exponential_backoff = 5;
                } catch (Throwable ex) {
                    LOG.warn(ex);

                    if (ex instanceof InterruptedException) {
                        break;
                    }

                    try {
                        Thread.sleep(exponential_backoff);

                        exponential_backoff *= 2;
                        if (exponential_backoff > 5000) {
                            exponential_backoff = 5000;
                        }
                    } catch (InterruptedException e) {
                        LOG.warn(ex);
                        break;
                    }
                }
            }
        } finally {
            this.isRunning.set(false);
        }
    }

    protected abstract void work() throws InterruptedException;
}
