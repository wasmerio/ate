package com.tokera.ate.io.repo;

import com.tokera.ate.dto.msg.MessageDataMetaDto;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.*;
public class DataGraphNode {

    public final MessageDataMetaDto     msg;
    public final String                 key;
    public final UUID                   version;
    public final @Nullable UUID         previousVersion;
    public @Nullable DataGraphNode      parentNode;
    public final Set<UUID>              mergesVersions;

    public DataGraphNode(MessageDataMetaDto msg) {
        this.msg = msg;
        this.version = msg.version();
        this.key = msg.getMeta().getKey();
        this.previousVersion =  msg.getData().getHeader().getPreviousVersion();
        this.mergesVersions = msg.getData().getHeader().getMerges();
    }
}