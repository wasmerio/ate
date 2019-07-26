package com.tokera.ate.delegates;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.base.BaseDaoInternal;
import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.DataStagingManager;
import com.tokera.ate.providers.PartitionKeySerializer;
import com.tokera.ate.scopes.Startup;
import org.apache.commons.lang.exception.ExceptionUtils;
import org.apache.kafka.clients.consumer.ConsumerRecord;
import org.apache.kafka.clients.producer.ProducerRecord;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.ApplicationScoped;
import java.util.UUID;

/**
 * Delegate used to perform some extra logging for debug purposes
 */
@Startup
@ApplicationScoped
public class DebugLoggingDelegate {
    AteDelegate d = AteDelegate.get();

    public enum CallbackDataType {
        Created,
        Update,
        Removed
    }

    public void logMergeDeferred(DataStagingManager staging) {
        if (d.bootstrapConfig.isLoggingWrites()) {
            StringBuilder sb = new StringBuilder();
            sb.append("merge_deferred: [cnt=");
            sb.append(staging.size());
            sb.append("]");

            if (d.bootstrapConfig.isLoggingWithStackTrace()) {
                String fullStackTrace = ExceptionUtils.getFullStackTrace(new Throwable());
                sb.append("\n");
                sb.append(fullStackTrace);
            }
            logInfo(sb.toString());
        }
    }

    public void logCallbackHook(String prefix, IPartitionKey partitionKey, Class<? extends BaseDao> objType, Class<?> callbackClazz) {
        if (d.bootstrapConfig.isLoggingCallbacks()) {
            StringBuilder sb = new StringBuilder();
            sb.append(prefix);
            sb.append(": [partition=");
            sb.append(PartitionKeySerializer.toString(partitionKey));
            sb.append(", type=");
            sb.append(objType.getSimpleName());
            sb.append(", callback=");
            sb.append(callbackClazz.getSimpleName());
            sb.append("]");
            logInfo(sb.toString());
        }
    }

    public void logCallbackData(String prefix, IPartitionKey partitionKey, UUID id, CallbackDataType action, Class<?> callbackClazz, @Nullable BaseDao obj) {
        if (d.bootstrapConfig.isLoggingCallbacks()) {
            StringBuilder sb = new StringBuilder();
            sb.append(prefix);
            sb.append(": [data partition=");
            sb.append(PartitionKeySerializer.toString(partitionKey));
            sb.append(", id=");
            sb.append(id);
            if (obj != null) {
                sb.append(", type=");
                sb.append(obj.getClass().getSimpleName());
            }
            sb.append(", action=");
            sb.append(action);
            sb.append(", callback=");
            sb.append(callbackClazz.getSimpleName());
            sb.append("]");

            if (d.bootstrapConfig.isLoggingCallbackData() && obj != null) {
                sb.append("\n");
                sb.append(d.yaml.serializeObj(obj));
            }

            logInfo(sb.toString());
        }
    }

    public void logRooted(IPartitionKey partitionKey, UUID id, String entityType, String keyHash)
    {
        if (d.bootstrapConfig.isLoggingChainOfTrust()) {
            logInfo("[" + partitionKey + "] chain-of-trust rooted: " + entityType + ":" + id + " on " + keyHash);
        }
    }

    public void logClaimed(IPartitionKey partitionKey, UUID id, String entityType)
    {
        if (d.bootstrapConfig.isLoggingChainOfTrust()) {
            logInfo("[" + partitionKey + "] chain-of-trust claimed: " + entityType + ":" + id);
        }
    }

    public void seedingPartitionStart(IPartitionKey partitionKey) {
        if (d.bootstrapConfig.isLoggingChainOfTrust()) {
            StringBuilder sb = new StringBuilder();
            sb.append("seeding_partition_start: [");
            sb.append(PartitionKeySerializer.toString(partitionKey));
            sb.append("]");
            logInfo(sb.toString());
        }
    }

    public void seedingPartitionEnd(IPartitionKey partitionKey) {
        if (d.bootstrapConfig.isLoggingChainOfTrust()) {
            StringBuilder sb = new StringBuilder();
            sb.append("seeding_partition_end: [");
            sb.append(PartitionKeySerializer.toString(partitionKey));
            sb.append("]");
            logInfo(sb.toString());
        }
    }

    public void logLoadingPartition(IPartitionKey key) {
        if (d.bootstrapConfig.isLoggingChainOfTrust()) {
            logInfo("loading-partition: " + key.partitionTopic() + ":" + key.partitionIndex());
        }
    }

    public void logDelete(IPartitionKey part, MessageDataDto data) {
        if (d.bootstrapConfig.isLoggingDeletes()) {
            StringBuilder sb = new StringBuilder();
            sb.append("remove: [->");
            sb.append(part);
            sb.append(":");
            sb.append(data.getHeader().getId());
            sb.append("]");
            if (d.bootstrapConfig.isLoggingMessages()) {
                sb.append("\n");
                sb.append(d.yaml.serializeObj(data));
            }
            logInfo(sb.toString());
        }
    }

    public void logDelete(BaseDao entity) {
        if (d.bootstrapConfig.isLoggingDeletes()) {
            StringBuilder sb = new StringBuilder();
            sb.append("remove: [->");
            sb.append(entity.addressableId());
            sb.append("]");
            if (d.bootstrapConfig.isLoggingData()) {
                sb.append("\n");
                sb.append(d.yaml.serializeObj(entity));
            }
            logInfo(sb.toString());
        }
    }

    public void logTrustValidationException(Throwable ex) {
        if (d.bootstrapConfig.isLoggingChainOfTrust()) {
            logWarn(ex);
        }
    }

    public void logMerge(@Nullable MessageDataDto data, @Nullable BaseDao entity, boolean later)
    {
        if (d.bootstrapConfig.isLoggingWrites()) {
            MessageDataHeaderDto header = data != null ? data.getHeader() : null;

            StringBuilder sb = new StringBuilder();

            if (later) {
                sb.append("write_later:");
            } else {
                sb.append("write_now:");
            }

            UUID id = header != null ? header.getId() : (entity != null ? entity.getId() : null);
            if (id != null) {
                sb.append(" [->");
                sb.append(id);
                sb.append("]");
            }

            String payloadClazz = header != null ? header.getPayloadClazz() : (entity != null ? BaseDaoInternal.getType(entity) : null);
            if (payloadClazz != null) {
                sb.append(" ");
                sb.append(payloadClazz);
            }

            UUID parentId = header != null ? header.getParentId() : (entity != null ? entity.getParentId() : null);
            if (parentId != null) {
                sb.append(" parent=");
                sb.append(parentId);
            }
            if (d.bootstrapConfig.isLoggingMessages() && data != null) {
                sb.append("\n");
                sb.append(d.yaml.serializeObj(data));
            }
            if (d.bootstrapConfig.isLoggingData() && entity != null) {
                sb.append("\n");
                sb.append(d.yaml.serializeObj(entity));
            }
            logInfo(sb.toString());
        }
    }

    public void logTrust(IPartitionKey part, MessagePublicKeyDto trustedKey) {
        if (d.bootstrapConfig.isLoggingChainOfTrust()) {
            StringBuilder sb = new StringBuilder();
            sb.append("trust: [->");
            sb.append(part);
            sb.append(":");
            sb.append(trustedKey.getPublicKeyHash());
            sb.append("] ");

            if (trustedKey instanceof MessagePrivateKeyDto) {
                sb.append("privateKey");
            } else {
                sb.append("publicKey");
            }

            if (d.bootstrapConfig.isLoggingMessages()) {
                sb.append("\n");
                sb.append(d.yaml.serializeObj(trustedKey));
            }

            logInfo(sb.toString());
        }
    }

    public void logTrust(IPartitionKey part, MessageDataDto data)
    {
        if (d.bootstrapConfig.isLoggingChainOfTrust()) {
            logTrust(part, data.getHeader());
        }
    }

    public void logCastle(IPartitionKey part, MessageSecurityCastleDto castle) {
        if (d.bootstrapConfig.isLoggingChainOfTrust()) {
            StringBuilder sb = new StringBuilder();
            sb.append("castle: [->");
            sb.append(part);
            sb.append("] id: ");
            sb.append(castle.getIdOrThrow());

            if (d.bootstrapConfig.isLoggingMessages()) {
                sb.append("\n");
                sb.append(d.yaml.serializeObj(castle));
            }

            logInfo(sb.toString());
        }
    }

    public void logTrust(IPartitionKey part, MessageDataHeaderDto header)
    {
        if (d.bootstrapConfig.isLoggingChainOfTrust()) {
            StringBuilder sb = new StringBuilder();
            sb.append("trust: [->");
            sb.append(part);
            sb.append("] data_commit: ");
            sb.append(header.getPayloadClazz());
            sb.append(":");
            sb.append(header.getId());

            sb.append(" attached to ");
            sb.append(header.getParentId());

            if (d.bootstrapConfig.isLoggingMessages()) {
                sb.append("\n");
                sb.append(d.yaml.serializeObj(header));
            }

            logInfo(sb.toString());
        }
    }

    public void logReceive(MessageBaseDto msg)
    {
        if (d.bootstrapConfig.isLoggingMessages()) {
            logInfo("rcv:\n" + d.yaml.serializeObj(msg));
        }
    }

    public void logSyncStart(MessageSyncDto sync)
    {
        if (d.bootstrapConfig.isLoggingSync()) {
            logInfo("sync_start (" + sync.getTicket1() + ":" + sync.getTicket2() + ")");
        }
    }

    public void logSyncMiss(MessageSyncDto sync)
    {
        if (d.bootstrapConfig.isLoggingSync()) {
            logInfo("sync_miss (" + sync.getTicket1() + ":" + sync.getTicket2() + ")");
        }
    }

    public void logSyncFinish(MessageSyncDto sync)
    {
        if (d.bootstrapConfig.isLoggingSync()) {
            logInfo("sync_finish (" + sync.getTicket1() + ":" + sync.getTicket2() + ")");
        }
    }

    public void logSyncWake(MessageSyncDto sync)
    {
        if (d.bootstrapConfig.isLoggingSync()) {
            logInfo("sync_wake (" + sync.getTicket1() + ":" + sync.getTicket2() + ")");
        }
    }

    public void logKafkaRecord(ConsumerRecord<String, MessageBase> record) {
        if (d.bootstrapConfig.isLoggingKafka()) {
            StringBuilder sb = new StringBuilder();

            sb.append("kafka_rcv(topic=");
            sb.append(record.topic());
            sb.append(", partition=");
            sb.append(record.partition());
            sb.append(", id=");
            sb.append(record.key());
            sb.append(", size=");
            sb.append(record.serializedValueSize());
            sb.append(")");

            logInfo(sb.toString());
        }
    }

    public void logKafkaSend(ProducerRecord<String, MessageBase> record, @Nullable MessageBaseDto msg) {
        if (d.bootstrapConfig.isLoggingKafka()) {
            StringBuilder sb = new StringBuilder();

            sb.append("kafka_send(topic=");
            sb.append(record.topic());
            sb.append(", partition=");
            sb.append(record.partition());
            sb.append(", id=");
            sb.append(record.key());
            if (msg != null) {
                sb.append(", type=");
                sb.append(msg.getClass().getSimpleName());
            }
            sb.append(")");

            logInfo(sb.toString());
        }
    }

    public void logInfo(String info) {
        System.out.println(info);
    }

    public void logWarn(Throwable ex) {
        String msg = ex.getMessage();
        if (msg != null) {
            System.err.println(ex.getClass().getName() + " - " + ex.getMessage());
        } else {
            System.err.println(ex.getClass().getName());
        }
        d.genericLogger.warn(ex);
    }
}
