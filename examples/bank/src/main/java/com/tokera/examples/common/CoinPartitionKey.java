package com.tokera.examples.common;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.providers.PartitionKeySerializer;

public class CoinPartitionKey implements IPartitionKey {
    @JsonIgnore
    private transient String base64;

    @Override
    public String partitionTopic() {
        return "coins";
    }

    @Override
    public int partitionIndex() {
        return 0;
    }

    @Override
    public DataPartitionType partitionType() {
        return DataPartitionType.Dao;
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
