package com.tokera.ate.io.ram;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.io.repo.IDataTopicBridge;

import javax.enterprise.context.ApplicationScoped;
import java.util.concurrent.ConcurrentHashMap;

@ApplicationScoped
public class RamBridgeBuilder {
    private AteDelegate d = AteDelegate.get();
    private final ConcurrentHashMap<String, RamTopicBridge> ramTopics;

    public RamBridgeBuilder() {
        this.ramTopics = new ConcurrentHashMap<>();
    }

    public IDataTopicBridge build(String topic, DataPartitionType type) {
        return ramTopics.computeIfAbsent(topic, t -> new RamTopicBridge(topic, type));
    }
}
