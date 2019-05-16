package com.tokera.ate.io.api;

import java.util.UUID;

/**
 * Interface used to resolve data objects into a partition mapping. This is used to programmatically
 * split data domains into clean partitions for performance and scalability reasons. It is important
 * that the implementation of this interface ensure the partitions are evenly spread and that consistency
 * between the partitions is handled by the application business logic.
 */
public interface IPartitionKeyMapper {

    /**
     * Maps a data object to a particular partition
     * @param id ID of the object to be mapped to a partition key
     * @return The topic and partition that this data object is related to
     */
    IPartitionKey resolve(UUID id);
}
