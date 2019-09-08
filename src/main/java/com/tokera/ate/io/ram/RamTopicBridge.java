package com.tokera.ate.io.ram;

import com.tokera.ate.dao.GenericPartitionKey;
import com.tokera.ate.dao.MessageBundle;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.kafka.KafkaTopicBridge;
import com.tokera.ate.io.kafka.KafkaTopicFactory;
import com.tokera.ate.io.repo.DataPartitionChain;
import com.tokera.ate.io.repo.IDataPartitionBridge;
import com.tokera.ate.io.repo.IDataTopicBridge;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.ws.rs.WebApplicationException;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.stream.Collectors;

/**
 * Represents a bridge of a particular partition with an in memory RAM copy of the data
 */
public class RamTopicBridge implements IDataTopicBridge {
    private final AteDelegate d = AteDelegate.get();
    private final String topic;
    private final DataPartitionType type;

    public RamTopicBridge(String topic, DataPartitionType type) {
        this.topic = topic;
        this.type = type;
    }

    @Override
    public IDataPartitionBridge createPartition(IPartitionKey key) {
        if (key.partitionTopic().equals(topic) ==false) {
            throw new WebApplicationException("Partition key does not match this topic.");
        }
        if (key.partitionIndex() >= KafkaTopicFactory.maxPartitionsPerTopic) {
            throw new WebApplicationException("Partition index can not exceed the maximum of " + KafkaTopicFactory.maxPartitionsPerTopic + " per topic.");
        }

        GenericPartitionKey wrapKey = new GenericPartitionKey(key);
        DataPartitionChain chain = new DataPartitionChain(key);
        RamPartitionBridge ret = new RamPartitionBridge(this, chain, type);

        ret.feed(d.ramDataRepository.read(wrapKey));
        ret.idle();

        return ret;
    }
}