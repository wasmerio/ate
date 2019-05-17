package com.tokera.ate.io.api;

/**
 * Represents a partition within the distributed commit log
 */
public interface IPartitionKey {

    /**
     * @return Name of the topic within the distributed commit log
     */
    String partitionTopic();

    /**
     * @return Index of the partition within this topic
     */
    int partitionIndex();
}