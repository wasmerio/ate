package com.tokera.ate.dao.io;

import com.tokera.server.api.delegate.MegaDelegate;
import com.tokera.server.api.dto.msg.MessageSyncDto;
import com.tokera.server.api.scope.TokenScoped;

import java.util.concurrent.ConcurrentLinkedQueue;

@TokenScoped
public class SyncIO {

    private MegaDelegate d = MegaDelegate.getUnsafe();

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

            d.io.sync(sync);
        }
    }
}
