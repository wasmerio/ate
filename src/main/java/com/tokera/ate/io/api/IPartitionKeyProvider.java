package com.tokera.ate.io.api;

public interface IPartitionKeyProvider {

    /**
     * @return Returns a partition key for the particular scope of context
     */
    IPartitionKey partitionKey();
}
