package com.tokera.ate.dao;

import com.tokera.ate.dao.msg.MessageBase;

public class MessageBundle {
    public final int partition;
    public final long offset;
    public final MessageBase msg;

    public MessageBundle(int partition, long offset, MessageBase msg) {
        this.partition = partition;
        this.offset = offset;
        this.msg = msg;
    }
}
