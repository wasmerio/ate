package com.tokera.ate.io.core;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageSyncDto;
import com.tokera.ate.events.TokenStateChangedEvent;
import com.tokera.ate.scopes.TokenScoped;

import javax.enterprise.event.Observes;
import java.util.concurrent.ConcurrentLinkedQueue;

/**
 * Coordinator that ensures pending transactions will be synchronized at user-defined points in the program
 */
@TokenScoped
public class TransactionCoordinator  {
    private AteDelegate d = AteDelegate.get();

    private ConcurrentLinkedQueue<MessageSyncDto> syncs = new ConcurrentLinkedQueue<>();

    public void add(MessageSyncDto sync)
    {
        syncs.add(sync);
    }

    public void finish()
    {
        while (true) {
            MessageSyncDto sync = syncs.poll();
            if (sync == null) return;
            d.headIO.sync(sync);
        }
    }

    public void onTokenChange(@Observes TokenStateChangedEvent event) {

        // Make sure any outstanding sync operations are fully executed for this topic
        if (d.currentToken.getWithinTokenScope()) {
            d.transaction.finish();
        }
    }
}
