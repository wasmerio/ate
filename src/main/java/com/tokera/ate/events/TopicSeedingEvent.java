package com.tokera.ate.events;

import com.tokera.ate.io.repo.DataPartition;
import com.tokera.ate.io.repo.DataPartitionChain;

public class TopicSeedingEvent {
    private DataPartition topic;
    private DataPartitionChain chain;

    public TopicSeedingEvent(DataPartition topic, DataPartitionChain chain) {
        this.topic = topic;
        this.chain = chain;
    }

    public DataPartition getTopic() {
        return topic;
    }

    public void setTopic(DataPartition topic) {
        this.topic = topic;
    }

    public DataPartitionChain getChain() {
        return chain;
    }

    public void setChain(DataPartitionChain chain) {
        this.chain = chain;
    }
}
