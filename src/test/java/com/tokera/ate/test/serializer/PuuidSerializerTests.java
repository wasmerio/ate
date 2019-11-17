package com.tokera.ate.test.serializer;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.delegates.YamlDelegate;
import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.extensions.YamlTagDiscoveryExtension;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.providers.PartitionKeySerializer;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;

import java.math.BigInteger;
import java.util.UUID;

@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public class PuuidSerializerTests {

    public class FakePartitionKey implements IPartitionKey {
        private String partitionTopic;
        private int partitionIndex;
        @JsonIgnore
        private transient String base64;

        public FakePartitionKey(String partitionTopic, int partitionIndex) {
            this.partitionTopic = partitionTopic;
            this.partitionIndex = partitionIndex;
        }

        @Override
        public String partitionTopic() {
            return partitionTopic;
        }

        @Override
        public int partitionIndex() {
            return partitionIndex;
        }

        @Override
        public DataPartitionType partitionType() { return DataPartitionType.Dao; }

        @Override
        public String toString() {
            return PartitionKeySerializer.toString(this);
        }

        @Override
        public int hashCode() {
            return PartitionKeySerializer.hashCode(this);
        }

        @Override
        public boolean equals(Object val) {
            return PartitionKeySerializer.equals(this, val);
        }

        @Override
        public String asBase64() {
            if (base64 != null) return base64;
            base64 = PartitionKeySerializer.serialize(this);
            return base64;
        }
    }

    @Test
    public void base16Test() {
        PUUID pid = PUUID.from(new FakePartitionKey("testdomain.com", 1), UUID.randomUUID());
        String val = pid.toBase16();
        PUUID pid2 = PUUID.fromBase16(val);
        Assertions.assertEquals(pid, pid2);
    }

    @Test
    public void base26Test() {
        PUUID pid = PUUID.from(new FakePartitionKey("testdomain.com", 1), UUID.randomUUID());
        String val = pid.toBase26();
        PUUID pid2 = PUUID.fromBase26(val);
        Assertions.assertEquals(pid, pid2);
    }

    @Test
    public void base36Test() {
        PUUID pid = PUUID.from(new FakePartitionKey("testdomain.com", 1), UUID.randomUUID());
        String val = pid.toBase36();
        PUUID pid2 = PUUID.fromBase36(val);
        Assertions.assertEquals(pid, pid2);
    }

    @Test
    public void base64Test() {
        PUUID pid = PUUID.from(new FakePartitionKey("testdomain.com", 1), UUID.randomUUID());
        String val = pid.toBase64();
        PUUID pid2 = PUUID.fromBase64(val);
        Assertions.assertEquals(pid, pid2);
    }

    @Test
    public void bigIntegerTest() {
        PUUID pid = PUUID.from(new FakePartitionKey("testdomain.com", 1), UUID.randomUUID());
        BigInteger val = pid.toBigInteger();
        PUUID pid2 = PUUID.fromBigInteger(val);
        Assertions.assertEquals(pid, pid2);
    }

    @Test
    public void bytesTest() {
        PUUID pid = PUUID.from(new FakePartitionKey("testdomain.com", 1), UUID.randomUUID());
        byte[] val = pid.toBytes();
        PUUID pid2 = PUUID.fromBytes(val);
        Assertions.assertEquals(pid, pid2);
    }
}
