package com.tokera.ate.io.api;

import com.tokera.ate.dto.msg.MessageDataMetaDto;

public interface IHookFeed {

    void feed(IPartitionKey key, MessageDataMetaDto msg);

}
