package com.tokera.ate.io.api;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.MessageDataMetaDto;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.UUID;

public interface ITaskHandler {

    UUID id();

    /**
     * @return Partition key that the subscription is registered against
     */
    IPartitionKey partitionKey();

    /**
     * Feeds another data message into this task
     */
    void feed(MessageDataMetaDto msg);

    /**
     * @return Class that the subscription is listening for
     */
    Class<? extends BaseDao> clazz();

    /**
     * @return Token that this processor will run under when it receives events
     */
    @Nullable TokenDto token();

    /**
     * @return Returns true if the task is still active (callback is still valid)
     */
    boolean isActive();
}
