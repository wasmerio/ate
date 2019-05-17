package com.tokera.ate.dao;

import com.tokera.ate.common.StringTools;
import com.tokera.ate.io.api.IPartitionKey;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.io.Serializable;
import java.util.UUID;

public final class PUUID implements IPartitionKey, Serializable, Comparable<PUUID> {
    private static final long serialVersionUID = -642512169720015696L;
    private final String partitionTopic;
    private final int partitionIndex;
    private final UUID id;

    public PUUID(String topic, int index, long mostSigBits, long leastSigBits) {
        this.partitionTopic = topic;
        this.partitionIndex = index;
        this.id = new UUID(mostSigBits, leastSigBits);
    }

    public PUUID(String topic, int index, UUID id) {
        this.partitionTopic = topic;
        this.partitionIndex = index;
        this.id = id;
    }

    public PUUID(IPartitionKey key, long mostSigBits, long leastSigBits) {
        this.partitionTopic = key.partitionTopic();
        this.partitionIndex = key.partitionIndex();
        this.id = new UUID(mostSigBits, leastSigBits);
    }

    public PUUID(IPartitionKey key, UUID id) {
        this.partitionTopic = key.partitionTopic();
        this.partitionIndex = key.partitionIndex();
        this.id = id;
    }

    @Override
    public String partitionTopic() {
        return this.partitionTopic;
    }

    @Override
    public int partitionIndex() {
        return this.partitionIndex;
    }

    public UUID id() {
        return this.id;
    }

    @Override
    public int compareTo(PUUID pid) {
        int diff = this.partitionTopic.compareTo(pid.partitionTopic);
        if (diff != 0) return diff;
        diff = Integer.compare(this.partitionIndex, pid.partitionIndex);
        if (diff != 0) return diff;
        return this.id.compareTo(pid.id);
    }

    public int hashCode() {
        long hash = this.partitionTopic.hashCode() ^
                    Integer.hashCode(this.partitionIndex) ^
                    this.id.hashCode();
        return (int)(hash >> 32) ^ (int)hash;
    }

    public boolean equals(Object other) {
        if (null != other && other.getClass() == PUUID.class) {
            PUUID pid = (PUUID)other;
            return this.partitionTopic.equals(pid.partitionTopic) &&
                   this.partitionIndex == pid.partitionIndex &&
                   this.id.equals(pid.id);
        } else {
            return false;
        }
    }

    @Override
    public String toString() {
        return this.partitionTopic() + "-" + this.partitionIndex() + "-" + this.id().getMostSignificantBits() + "-" + this.id().getLeastSignificantBits();
    }

    public static String toString(@Nullable PUUID pid) {
        if (pid == null) return "null";
        return pid.toString();
    }

    public static @Nullable PUUID parse(@Nullable String _val) {
        String val = StringTools.makeOneLineOrNull(_val);
        val = StringTools.specialParse(val);
        if (val == null || val.length() <= 0) return null;

        String[] comps = val.split("-");
        if (comps.length != 4) return null;

        return new PUUID(
                comps[0],
                Integer.parseInt(comps[1]),
                Long.parseLong(comps[2]),
                Long.parseLong(comps[3]));
    }
}
