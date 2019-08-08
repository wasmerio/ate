package com.tokera.ate.dao;

import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.providers.PartitionKeySerializer;

import javax.enterprise.context.Dependent;
import java.io.Serializable;

@Dependent
@YamlTag("generic.partition.key")
public class GenericPartitionKey implements IPartitionKey, Serializable {
    private static final long serialVersionUID = -8032836543927736149L;

    private final String topic;
    private final int partition;
    private final DataPartitionType type;

    public GenericPartitionKey(String topic, int partition, DataPartitionType type) {
        this.topic = topic;
        this.partition = partition;
        this.type = type;
    }

    public GenericPartitionKey(IPartitionKey key) {
        this.topic = key.partitionTopic();
        this.partition = key.partitionIndex();
        this.type = key.partitionType();
    }

    @Override
    public String partitionTopic() {
        return topic;
    }

    @Override
    public int partitionIndex() {
        return partition;
    }

    @Override
    public DataPartitionType partitionType() { return type; }

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
