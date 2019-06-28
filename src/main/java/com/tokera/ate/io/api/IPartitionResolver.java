package com.tokera.ate.io.api;

import com.tokera.ate.dao.IRights;
import com.tokera.ate.dao.base.BaseDao;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.UUID;

/**
 * Interface used to resolve data objects into a partition mapping. This is used to programmatically
 * split data domains into clean partitions for performance and scalability reasons. It is important
 * that the implementation of this interface ensure the partitions are evenly spread and that consistency
 * between the partitions is handled by the application business logic.
 */
public interface IPartitionResolver {

    /**
     * Maps a data object to a particular partition
     * @param obj Reference to the data object to be mapped
     * @return The topic and partition that this data object is related to
     */
    IPartitionKey resolveOrThrow(BaseDao obj);

    /**
     * Maps a data object to a particular partition
     * @param obj Reference to the data object to be mapped
     * @return The topic and partition that this data object is related to
     */
    @Nullable IPartitionKey resolveOrNull(BaseDao obj);

    /**
     * Maps a rights interface to a particular partition
     * @param obj Reference to the rights interface to be mapped
     * @return The topic and partition that this data object is related to
     */
    IPartitionKey resolveOrThrow(IRights obj);

    /**
     * Maps a rights interface to a particular partition
     * @param obj Reference to the rights interface to be mapped
     * @return The topic and partition that this data object is related to
     */
    @Nullable IPartitionKey resolveOrNull(IRights obj);
}
