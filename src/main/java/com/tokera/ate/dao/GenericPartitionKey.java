package com.tokera.ate.dao;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonTypeName;
import com.fasterxml.jackson.databind.annotation.JsonDeserialize;
import com.fasterxml.jackson.databind.annotation.JsonSerialize;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.providers.*;

import javax.enterprise.context.Dependent;
import java.io.Serializable;

@Dependent
@YamlTag("gpkey")
@JsonTypeName("gpkey")
@JsonSerialize(using = GenericPartitionKeyJsonSerializer.class)
@JsonDeserialize(using = GenericPartitionKeyJsonDeserializer.class)
public final class GenericPartitionKey implements IPartitionKey, Serializable, Comparable<GenericPartitionKey> {
    private static final long serialVersionUID = -8032836543927736149L;

    private String topic;
    private int partition;
    private DataPartitionType type;
    @JsonIgnore
    private transient String base64;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public GenericPartitionKey() {
    }

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

    @Override
    public String asBase64() {
        if (base64 != null) return base64;
        base64 = PartitionKeySerializer.serialize(this);
        return base64;
    }

    @Override
    public int compareTo(GenericPartitionKey other) {
        int diff = this.topic.compareTo(other.topic);
        if (diff != 0) return diff;
        diff = Integer.compare(this.partition, other.partition);
        if (diff != 0) return diff;
        diff = this.type.compareTo(other.type);
        return diff;
    }
}
