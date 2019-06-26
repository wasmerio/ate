package com.tokera.ate.io.api;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.TokenDto;
import org.checkerframework.checker.nullness.qual.Nullable;

public interface ITask {

    /**
     * @return Partition key that the subscription is registered against
     */
    IPartitionKey partitionKey();

    /**
     * @return Class that the subscription is listening for
     */
    Class<? extends BaseDao> clazz();

    /**
     * @return Processor that will deal with these data objects as they are processed
     */
    ITaskCallback<? extends BaseDao> callback();

    /**
     * @return Token that this processor will run under when it receives events
     */
    @Nullable TokenDto token();
}
