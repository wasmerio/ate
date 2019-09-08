package com.tokera.ate.io.ram;

import com.tokera.ate.dao.MessageBundle;
import com.tokera.ate.dao.TopicAndPartition;
import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.io.api.IPartitionKey;

import javax.enterprise.context.ApplicationScoped;
import java.util.ArrayList;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicLong;

@ApplicationScoped
public class RamDataRepository {
    private final ConcurrentHashMap<TopicAndPartition, AtomicLong> offsets = new ConcurrentHashMap<>();
    private final ConcurrentHashMap<TopicAndPartition, ArrayList<MessageBundle>> data = new ConcurrentHashMap<>();

    private ArrayList<MessageBundle> partition(TopicAndPartition where) {
        return data.computeIfAbsent(where, k -> new ArrayList<>());
    }

    public MessageBundle write(TopicAndPartition where, MessageBase msg) {
        long offset = offsets.computeIfAbsent(where, a -> new AtomicLong(1L)).incrementAndGet();
        MessageBundle bundle = new MessageBundle(where.partitionIndex(), offset, msg);
        partition(where).add(bundle);
        return bundle;
    }

    public Iterable<MessageBundle> read(TopicAndPartition where) {
        return data.getOrDefault(where, new ArrayList<>());
    }

    public Iterable<MessageBundle> read(IPartitionKey key) {
        TopicAndPartition where = new TopicAndPartition(key);
        return data.getOrDefault(where, new ArrayList<>());
    }
}
