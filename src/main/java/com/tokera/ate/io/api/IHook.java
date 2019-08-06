package com.tokera.ate.io.api;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.MessageDataMetaDto;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.UUID;

public interface IHook {

    UUID id();

    IPartitionKey partitionKey();

    void feed(MessageDataMetaDto msg);

    boolean isActive();
}
