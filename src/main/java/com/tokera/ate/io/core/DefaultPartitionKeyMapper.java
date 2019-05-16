package com.tokera.ate.io.core;

import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.api.IPartitionKeyMapper;
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
        private final static int maxTopics = 10000;
        private final static int maxPartitions = 200000;
        private final int maxPartitionsPerTopic = maxPartitions / maxTopics;

        public Murmur2BasedPartitioningStrategy(UUID id) {
            ByteBuffer bb = ByteBuffer.wrap(new byte[16]);
            bb.putLong(id.getMostSignificantBits());
            bb.putLong(id.getLeastSignificantBits());
            this.hash = Utils.murmur2(bb.array());
        }

        @Override
        public String partitionTopic() {
            return String.format("data%d", hash % maxTopics);
        }

        @Override
        public int partitionIndex() {
            return (hash / maxTopics) % maxPartitionsPerTopic;
        }

        @Override
        public int maxPartitionsPerTopic() {
            return maxPartitionsPerTopic;
        }
    }

    @Override
    public IPartitionKey resolve(UUID id) {
        return new Murmur2BasedPartitioningStrategy(id);
    }
}