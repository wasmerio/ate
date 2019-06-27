package com.tokera.ate.dao;

import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.io.api.IPartitionKey;

import javax.enterprise.context.Dependent;
import java.io.Serializable;

@Dependent
@YamlTag("generic.partition.key")
public class GenericPartitionKey implements IPartitionKey, Serializable {
    private static final long serialVersionUID = -8032836543927736149L;

    private final String topic;
    private final int partition;

    public GenericPartitionKey(String topic, int partition) {
        this.topic = topic;
        this.partition = partition;
    }

    @Override
    public String partitionTopic() {
        return topic;
    }

    @Override
    public int partitionIndex() {
        return partition;
    }
}
