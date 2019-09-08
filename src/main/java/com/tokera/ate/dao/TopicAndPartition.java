package com.tokera.ate.dao;

import com.fasterxml.jackson.annotation.JsonTypeName;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.io.api.IPartitionKey;

import javax.enterprise.context.Dependent;
import java.io.Serializable;

@Dependent
@YamlTag("topicpart")
@JsonTypeName("topicpart")
public final class TopicAndPartition implements Serializable, Comparable<TopicAndPartition> {
    private static final long serialVersionUID = -4780665965525636535L;

    private String topic;
    private int partition;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public TopicAndPartition() {
    }

    public TopicAndPartition(String topic, int partition) {
        this.topic = topic;
        this.partition = partition;
    }

    public TopicAndPartition(IPartitionKey key) {
        this.topic = key.partitionTopic();
        this.partition = key.partitionIndex();
    }

    public String partitionTopic() {
        return topic;
    }

    public int partitionIndex() {
        return partition;
    }

    @Override
    public String toString() {
        return topic + "-" + partition;
    }

    @Override
    public int hashCode() {
        return toString().hashCode();
    }

    @Override
    public boolean equals(Object val) {
        if (val instanceof TopicAndPartition) {
            TopicAndPartition other = (TopicAndPartition)val;
            return this.partition == other.partition &&
                   this.topic.equals(other.topic);
        }
        return false;
    }

    @Override
    public int compareTo(TopicAndPartition other) {
        int diff = this.topic.compareTo(other.topic);
        if (diff != 0) return diff;
        diff = Integer.compare(this.partition, other.partition);
        return diff;
    }
}
