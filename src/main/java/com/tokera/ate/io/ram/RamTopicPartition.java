package com.tokera.ate.io.ram;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.dao.GenericPartitionKey;
import com.tokera.ate.dto.msg.MessageBaseDto;
import com.tokera.ate.io.api.IPartitionKey;

import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicLong;

public class RamTopicPartition
{
    public LoggerHook LOG;
    public Integer number;
    public IPartitionKey partitionKey;
    public AtomicLong offsetSeed;
    public ConcurrentHashMap<Long, MessageBaseDto> messages;

    public RamTopicPartition(GenericPartitionKey key) {
        this.LOG = new LoggerHook(RamTopicPartition.class);
        this.number = 0;
        this.offsetSeed = new AtomicLong();
        this.messages = new ConcurrentHashMap<>();
        this.partitionKey = key;
    }
}
