package com.tokera.ate.io.api;

import com.fasterxml.jackson.databind.annotation.JsonDeserialize;
import com.fasterxml.jackson.databind.annotation.JsonSerialize;
import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.providers.PartitionKeyJsonDeserializer;
import com.tokera.ate.providers.PartitionKeyJsonSerializer;
import com.tokera.ate.providers.PartitionKeySerializer;

/**
 * Represents a partition within the distributed commit log
 */
@JsonSerialize(using = PartitionKeyJsonSerializer.class)
@JsonDeserialize(using = PartitionKeyJsonDeserializer.class)
public interface IPartitionKey {

    /**
     * @return Name of the topic within the distributed commit log
     */
    String partitionTopic();

    /**
     * @return Index of the partition within this topic
     */
    int partitionIndex();

    /**
     * @return Type of partition that should be created
     */
    DataPartitionType partitionType();

    /**
     * @return Returns this partition key as a Base64 version
     */
    default String asBase64() {
        return PartitionKeySerializer.serialize(this);
    }
}