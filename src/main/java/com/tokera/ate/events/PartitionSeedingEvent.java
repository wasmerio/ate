package com.tokera.ate.events;

import com.tokera.ate.io.repo.DataPartition;
import com.tokera.ate.io.repo.DataPartitionChain;

public class PartitionSeedingEvent {
    private DataPartition partition;
    private DataPartitionChain chain;

    public PartitionSeedingEvent(DataPartition partition, DataPartitionChain chain) {
        this.partition = partition;
        this.chain = chain;
    }

    public DataPartition getPartition() {
        return partition;
    }

    public void setPartition(DataPartition partition) {
        this.partition = partition;
    }

    public DataPartitionChain getChain() {
        return chain;
    }

    public void setChain(DataPartitionChain chain) {
        this.chain = chain;
    }
}
