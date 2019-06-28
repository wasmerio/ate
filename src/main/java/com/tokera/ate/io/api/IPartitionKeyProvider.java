package com.tokera.ate.io.api;

public interface IPartitionKeyProvider {

    /**
     * @param shouldThrow Determines if the provider should throw an exception if its not found
     * @return Returns a partition key for the particular scope of context
     */
    IPartitionKey partitionKey(boolean shouldThrow);
}
