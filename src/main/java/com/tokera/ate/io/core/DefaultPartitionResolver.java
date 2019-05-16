package com.tokera.ate.io.core;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.api.IPartitionResolver;
import org.apache.kafka.common.utils.Utils;

import java.nio.ByteBuffer;
import java.util.UUID;

/**
 * Default implementation of the partition resolver which will use a hashing algorithm on the primary
 * key of the root of the tree to determine the partition that data will be mapped to.
 */
public class DefaultPartitionResolver implements IPartitionResolver {
    private AteDelegate d = AteDelegate.getUnsafe();

    @Override
    public IPartitionKey resolve(BaseDao obj) {

        for (;;) {
            if (obj instanceof IPartitionKey) {
                return (IPartitionKey) obj;
            }
            BaseDao next = d.daoHelper.getParent(obj);
            if (next == null) {
                if (obj.getParentId() != null) {
                    throw new RuntimeException("Unable to transverse up the tree high enough to determine the topic and partition for this data object [" + obj + "].");
                }
                return d.headIO.partitionKeyMapper().resolve(obj.getId());
            }
            obj = next;
        }
    }
}
