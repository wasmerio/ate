package com.tokera.ate.io.task;

import com.google.common.base.Stopwatch;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageDataHeaderDto;
import com.tokera.ate.dto.msg.MessageDataMetaDto;
import com.tokera.ate.io.api.IHook;
import com.tokera.ate.io.api.IHookCallback;
import com.tokera.ate.io.api.IPartitionKey;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.LinkedList;
import java.util.UUID;
import java.util.concurrent.TimeUnit;

public class PollHook implements IHookCallback {
    private final AteDelegate d = AteDelegate.get();
    private final PUUID objId;
    private final Class<? extends BaseDao> clazz;
    private final UUID id = UUID.randomUUID();
    private final IPartitionKey partitionKey;
    private final LinkedList<MessageDataMetaDto> msgs = new LinkedList<>();

    public PollHook(PUUID objId, Class<? extends BaseDao> clazz) {
        this.objId = objId;
        this.clazz = clazz;
        this.partitionKey = objId.partition();
    }

    @Override
    public UUID id() {
        return this.id;
    }

    @Override
    public void onData(MessageDataMetaDto msg, IHook hook) {
        MessageDataDto data = msg.getData();

        MessageDataHeaderDto header = data.getHeader();
        if (header.getIdOrThrow().equals(objId.id()) == false) {
            return;
        }

        synchronized (this) {
            msgs.add(msg);
            this.notifyAll();
        }
    }

    private @Nullable BaseDao process(MessageDataMetaDto msg)
    {
        BaseDao ret = d.dataSerializer.fromDataMessage(this.partitionKey, msg, true);
        if (ret != null) {
            d.io.currentTransaction().cache(this.partitionKey, ret);
        }
        return ret;
    }

    public @Nullable BaseDao poll(long timeout) {
        MessageDataMetaDto ret = null;
        synchronized (this)
        {
            Stopwatch timer = Stopwatch.createStarted();
            do {
                if (msgs.isEmpty() == false) {
                    ret = msgs.pop();
                    break;
                }

                long waitTime = Math.max(1, timeout - timer.elapsed(TimeUnit.MILLISECONDS));
                try {
                    this.wait(waitTime);
                } catch (InterruptedException e) {
                }

                if (msgs.isEmpty() == false) {
                    ret = msgs.pop();
                    break;
                }
            } while (timer.elapsed(TimeUnit.MILLISECONDS) <= timeout);
        }

        return process(ret);
    }
}
