package com.tokera.ate.io.core;

import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.api.IPartitionKeyMapper;
import com.tokera.ate.io.kafka.KafkaTopicBridge;
import com.tokera.ate.providers.PartitionKeySerializer;
import org.apache.kafka.common.utils.Utils;

import java.nio.ByteBuffer;
import java.util.UUID;

/**
 * Default implementation of the partition resolver which will use a hashing algorithm on the primary
 * key of the root of the tree to determine the partition that data will be mapped to.
 */
public class DefaultPartitionKeyMapper implements IPartitionKeyMapper {
    public class Murmur2BasedPartitioningStrategy implements IPartitionKey {
        private final int hash;

        public Murmur2BasedPartitioningStrategy(UUID id) {
            ByteBuffer bb = ByteBuffer.wrap(new byte[16]);
            bb.putLong(id.getMostSignificantBits());
            bb.putLong(id.getLeastSignificantBits());

            int hash = Utils.murmur2(bb.array());
            if (hash < 0) hash = -hash;
            if (hash < 0) hash = 0;
            this.hash = hash;
        }

        @Override
        public String partitionTopic() {
            return String.format("d%d", (hash / KafkaTopicBridge.maxPartitionsPerTopic) % KafkaTopicBridge.maxTopics);
        }

        @Override
        public int partitionIndex() {
            return hash % KafkaTopicBridge.maxPartitionsPerTopic;
        }

        @Override
        public String toString() {
            return PartitionKeySerializer.toString(this);
        }

        @Override
        public int hashCode() {
            return PartitionKeySerializer.hashCode(this);
        }

        @Override
        public boolean equals(Object val) {
            return PartitionKeySerializer.equals(this, val);
        }
    }

    @Override
    public IPartitionKey resolve(UUID id) {
        return new Murmur2BasedPartitioningStrategy(id);
    }

    @Override
    public int maxPartitionsPerTopic() {
        return KafkaTopicBridge.maxPartitionsPerTopic;
    }
}