package com.tokera.ate.io.api;

import java.util.Comparator;

public class PartitionKeyComparator implements Comparator<IPartitionKey> {

    @Override
    public int compare(IPartitionKey a, IPartitionKey b) {
        int diff = a.partitionTopic().compareTo(b.partitionTopic());
        if (diff != 0) return diff;
        return Integer.compare(a.partitionIndex(), b.partitionIndex());
    }
}
