package com.tokera.ate.io.kafka;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageSyncDto;
import com.tokera.ate.io.api.IPartitionKey;

import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import java.util.Random;
import java.util.concurrent.ConcurrentHashMap;

@ApplicationScoped
public class KafkaSync {
    protected AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;

    private final Random rand = new Random();
    private ConcurrentHashMap<MessageSyncDto, Object> syncs = new ConcurrentHashMap<>();

    public MessageSyncDto startSync() {
        MessageSyncDto sync = new MessageSyncDto(
                rand.nextLong(),
                rand.nextLong());
        startSync(sync, new Object());
        return sync;
    }

    public MessageSyncDto startSync(MessageSyncDto sync) {
        sync = new MessageSyncDto(sync);
        startSync( sync, new Object());
        return sync;
    }

    private void startSync(MessageSyncDto sync, Object waitOn) {
        syncs.put(sync, waitOn);
        d.debugLogging.logSyncStart(sync);
    }

    public boolean hasFinishSync(MessageSyncDto sync) {
        if (sync == null) return true;
        return syncs.containsKey(sync) == false;
    }

    public boolean finishSync(MessageSyncDto sync) {
        return finishSync(sync, 60000);
    }

    public boolean finishSync(MessageSyncDto sync, int timeout) {
        Object wait = MapTools.getOrNull(this.syncs, sync);
        if (wait == null) return true;

        synchronized (wait) {
            if (syncs.containsKey(sync) == false) {
                return true;
            }

            try {
                wait.wait(timeout);
                return hasFinishSync(sync);
            } catch (InterruptedException e) {
                return false;
            } finally {
                syncs.remove(sync);
            }
        }
    }

    public boolean sync() {
        return sync(60000);
    }

    public boolean sync(int timeout) {

        Object wait = new Object();
        synchronized (wait)
        {
            MessageSyncDto sync = new MessageSyncDto(
                    rand.nextLong(),
                    rand.nextLong());
            startSync(sync, wait);

            try {
                wait.wait(timeout);
                d.debugLogging.logSyncWake(sync);
                return hasFinishSync(sync);
            } catch (InterruptedException e) {
                return false;
            } finally {
                syncs.remove(sync);
            }
        }
    }

    public void processSync(MessageSyncDto sync)
    {
        d.debugLogging.logReceive(sync);

        Object wait = syncs.remove(sync);
        if (wait == null) {
            d.debugLogging.logSyncMiss(sync);
            return;
        }

        synchronized (wait) {
            d.debugLogging.logSyncFinish(sync);
            wait.notifyAll();
        }
    }
}
