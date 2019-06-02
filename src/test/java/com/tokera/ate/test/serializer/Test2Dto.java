package com.tokera.ate.test.serializer;

import com.tokera.ate.io.api.IPartitionKey;

public class Test2Dto {
    private IPartitionKey share;

    public Test2Dto() {
    }

    public IPartitionKey getShare() {
        return share;
    }

    public void setShare(IPartitionKey key) {
        this.share = key;
    }
}
