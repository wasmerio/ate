package com.tokera.examples.common;

import com.tokera.ate.io.api.IPartitionKey;

public class CoinPartitionKey implements IPartitionKey {

    @Override
    public String partitionTopic() {
        return "coins";
    }

    @Override
    public int partitionIndex() {
        return 0;
    }
}
