package com.tokera.ate.events;

import com.tokera.ate.io.repo.DataTopic;
import com.tokera.ate.io.repo.DataTopicChain;

public class TopicSeedingEvent {
    private DataTopic topic;
    private DataTopicChain chain;

    public TopicSeedingEvent(DataTopic topic, DataTopicChain chain) {
        this.topic = topic;
        this.chain = chain;
    }

    public DataTopic getTopic() {
        return topic;
    }

    public void setTopic(DataTopic topic) {
        this.topic = topic;
    }

    public DataTopicChain getChain() {
        return chain;
    }

    public void setChain(DataTopicChain chain) {
        this.chain = chain;
    }
}
