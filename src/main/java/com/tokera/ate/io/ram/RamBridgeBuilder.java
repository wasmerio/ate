package com.tokera.ate.io.ram;

import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.IDataTopicBridge;

import javax.enterprise.context.ApplicationScoped;
import java.util.concurrent.ConcurrentHashMap;

@ApplicationScoped
public class RamBridgeBuilder {
    private final ConcurrentHashMap<String, RamTopicBridge> ramTopics;

    public RamBridgeBuilder() {
        this.ramTopics = new ConcurrentHashMap<>();
    }

    public IDataTopicBridge build(String topic, DataPartitionType type) {
        return new RamTopicBridge(topic, type);
    }

    public void destroyAll() {
        this.ramTopics.clear();
    }
}
