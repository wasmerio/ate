package com.tokera.ate.io.repo;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.dto.msg.MessageBaseDto;

import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicLong;

public class RamTopicPartition
{
    public LoggerHook LOG;
    public Integer number;
    public String topicName;
    public AtomicLong offsetSeed;
    public ConcurrentHashMap<Long, MessageBaseDto> messages;
    public ConcurrentHashMap<Long, Long> timestamps;

    public RamTopicPartition(String topicName) {
        this.LOG = new LoggerHook(RamTopicPartition.class);
        this.number = 0;
        this.offsetSeed = new AtomicLong();
        this.messages = new ConcurrentHashMap<>();
        this.timestamps = new ConcurrentHashMap<>();
        this.topicName = topicName;
    }
}
