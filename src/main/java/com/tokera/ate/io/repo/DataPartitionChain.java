package com.tokera.ate.io.repo;

import com.tokera.ate.dao.TopicAndPartition;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.kafka.MessageSerializer;
import com.tokera.ate.dao.msg.*;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.core.DataMaintenance;
import com.tokera.ate.providers.PartitionKeySerializer;
import com.tokera.ate.security.Encryptor;

import java.io.IOException;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentMap;

import com.tokera.ate.units.Hash;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.bouncycastle.crypto.InvalidCipherTextException;

/**
 * Represents a cryptographic verified graph of strongly typed data objects that form a chain-of-trust. These chains
 * are effectively the heart of the database.
 */
public class DataPartitionChain {
    private final AteDelegate d = AteDelegate.get();
    
    private final IPartitionKey key;
    private final TopicAndPartition what;
    private final DataMaintenance.State maintenanceState;
    private final ConcurrentMap<UUID, MessageDataHeaderDto> rootOfTrust;
    private final ConcurrentMap<UUID, DataContainer> chainOfTrust;
    private final ConcurrentMap<String, HashSet<UUID>> byClazz;
    private final ConcurrentMap<UUID, MessageSecurityCastleDto> castles;
    private final ConcurrentMap<String, MessagePublicKeyDto> publicKeys;
    private final Encryptor encryptor;
    
    public DataPartitionChain(IPartitionKey key) {
        this.key = key;
        this.what = new TopicAndPartition(key);
        this.maintenanceState = d.dataMaintenance.getOrCreateState(key);
        this.rootOfTrust = new ConcurrentHashMap<>();
        this.chainOfTrust = new ConcurrentHashMap<>();
        this.publicKeys = new ConcurrentHashMap<>();
        this.castles = new ConcurrentHashMap<>();
        this.byClazz = new ConcurrentHashMap<>();
        this.encryptor = Encryptor.getInstance();

        this.addTrustKey(d.encryptor.getTrustOfPublicRead().key());
        this.addTrustKey(d.encryptor.getTrustOfPublicWrite().key());
    }

    public IPartitionKey partitionKey() { return this.key; }

    public void addTrustDataHeader(MessageDataHeaderDto trustedHeader) {

        MessageDataDto data = new MessageDataDto(
                trustedHeader,
                null,
                null);

        d.debugLogging.logTrust(this.partitionKey(), trustedHeader);

        MessageMetaDto meta = new MessageMetaDto(
                UUID.randomUUID().toString(),
                0L,
                0L);

        this.addTrustData(data, meta, false);
    }
    
    public void addTrustKey(MessagePublicKeyDto trustedKey) {
        d.debugLogging.logTrust(this.partitionKey(), trustedKey);

        if (d.bootstrapConfig.isExtraValidation()) {
            d.validationUtil.validateOrThrow(trustedKey);
        }

        @Hash String trustedKeyHash = trustedKey.getPublicKeyHash();
        if (trustedKeyHash != null) {
            this.publicKeys.put(trustedKeyHash, trustedKey);
        }
    }

    @SuppressWarnings({"known.nonnull"})
    public void addTrustData(MessageDataDto data, MessageMetaDto meta, boolean invokeCallbacks) {
        d.debugLogging.logTrust(this.partitionKey(), data);

        // Get the ID
        MessageDataHeaderDto header = data.getHeader();
        UUID id = header.getIdOrThrow();

        // If it has no payload then strip it from the chain of trust
        if (data.hasPayload() == false) {
            this.chainOfTrust.remove(id);
            this.byClazz.compute(header.getPayloadClazzOrThrow(), (a, b) -> {
                if (b != null) b.remove(id);
                return b;
            });
            this.maintenanceState.dont_merge(id);
            this.maintenanceState.tombstone(meta.getKey());
            return;

        } else {
            this.maintenanceState.dont_tombstone(meta.getKey());
        }

        // Add it to the chain of trust
        DataContainer container = this.chainOfTrust.compute(id, (i, c) -> {
            if (c == null) c = new DataContainer(id, this.key);
            c.add(data, meta);
            this.byClazz.compute(header.getPayloadClazzOrThrow(), (a, b) -> {
                if (b == null) b = new HashSet<>();
                b.add(id);
                d.permissionCache.invalidate(header.getPayloadClazzOrThrow(), this.partitionKey(), id);
                return b;
            });
            return c;
        });

        // If the container requires a merge then notify the maintenance thread
        if (container.requiresMerge()) {
            this.maintenanceState.merge(container.id);
        } else {
            this.maintenanceState.dont_merge(container.id);
        }

        // Invoke the task manager so anything waiting for events will trigger
        if (invokeCallbacks) {
            d.taskManager.feed(this.partitionKey(), data, meta);
            d.hookManager.feed(this.partitionKey(), data, meta);
        }
    }
    
    public void addTrustCastle(MessageSecurityCastleDto castle, @Nullable LoggerHook LOG) {
        d.debugLogging.logCastle(this.partitionKey(), castle);

        this.castles.put(castle.getIdOrThrow(), castle);
    }
    
    public boolean rcv(MessageBaseDto msg, MessageMetaDto meta, boolean invokeCallbacks, @Nullable LoggerHook LOG) throws IOException, InvalidCipherTextException {
        if (msg == null) {
            drop(LOG, null, null, "unhandled message type");
            return false;
        }
        if (msg instanceof MessageDataDto) {
            return processData((MessageDataDto)msg, meta, invokeCallbacks, LOG);
        }
        if (msg instanceof MessagePublicKeyDto) {
            return processPublicKey((MessagePublicKeyDto)msg, LOG);
        }
        if (msg instanceof MessageSecurityCastleDto) {
            return processCastle((MessageSecurityCastleDto)msg, LOG);
        }
        if (msg instanceof MessageSyncDto) {
            return processSync((MessageSyncDto)msg, LOG);
        }
        
        drop(LOG, msg, meta, "unhandled message type");
        return false;
    }
    
    public void drop(@Nullable LoggerHook LOG, @Nullable MessageBaseDto msg, @Nullable MessageMetaDto meta, String why) {
        drop(LOG, msg, meta, why, null);
    }
    
    public void drop(@Nullable LoggerHook LOG, @Nullable MessageBaseDto msg, @Nullable MessageMetaDto meta, String why, @Nullable MessageDataHeader parentHeader) {
        if (d.bootstrapConfig.isLoggingMessageDrops()) {
            String index;
            if (meta != null) {
                index = "partition=" + PartitionKeySerializer.toString(this.partitionKey()) + ", offset=" + meta.getOffset();
            } else {
                index = "partition=" + PartitionKeySerializer.toString(this.partitionKey());
            }

            String err;
            if (msg instanceof MessageDataDto) {
                MessageDataDto data = (MessageDataDto) msg;
                drop(LOG, data, meta, why, parentHeader);
                return;
            } else if (msg != null) {
                err = "Dropping message on [" + index + "] - " + why + " [type=" + msg.getClass().getSimpleName() + "]";
            } else {
                err = "Dropping message on [" + index + "] - " + why;
            }

            if (LOG != null) {
                LOG.error(err);
            } else {
                new LoggerHook(DataPartitionChain.class).warn(err);
            }
        }
    }
    
    public void drop(@Nullable LoggerHook LOG, @Nullable MessageDataHeaderDto header, String why) {
        if (d.bootstrapConfig.isLoggingMessageDrops()) {
            String err;

            UUID id = header.getIdOrThrow();
            err = "Dropping data on [" + PartitionKeySerializer.toString(this.partitionKey()) + "] - " + why + " [clazz=" + header.getPayloadClazzOrThrow() + ", id=" + id + "]";

            if (LOG != null) {
                LOG.error(err);
            } else {
                new LoggerHook(DataPartitionChain.class).warn(err);
            }
        }
    }
    
    public boolean promoteChainEntry(MessageDataMetaDto msg, boolean invokeCallbacks, @Nullable LoggerHook LOG) {
        MessageDataDto data = msg.getData();

        // Validate the data
        if (validateTrustStructureAndWritabilityWithoutSavedData(data, LOG) == false) {
            return false;
        }
        
        // Add it to the trust tree and return success
        addTrustData(data, msg.getMeta(), invokeCallbacks);
        return true;
    }
    
    public boolean validate(MessageBaseDto msg, @Nullable LoggerHook LOG)
    {
        if (msg instanceof MessageDataDto) {
            return validateTrustStructureAndWritability((MessageDataDto)msg, LOG);
        } else {
            return true;
        }
    }

    public TrustValidatorBuilder createTrustValidator(@Nullable LoggerHook LOG) {
        return new TrustValidatorBuilder()
                .withLogger(LOG)
                .withFailureCallback(f -> this.drop(f.LOG, f.header, f.why))
                .withGetRootOfTrust(id -> this.getRootOfTrust(id))
                .withGetDataCallback(id -> this.getData(id))
                .withGetPublicKeyCallback(hash -> this.getPublicKey(hash));
    }

    public TrustValidatorBuilder createTrustValidatorIncludingStaging(@Nullable LoggerHook LOG) {
        return createTrustValidator(LOG)
                .withGetPublicKeyCallback(hash -> {
                    MessagePublicKeyDto ret = d.requestContext.currentTransaction().findPublicKey(this.key, hash);
                    if (ret != null) return ret;
                    return this.getPublicKey(hash);
                });
    }

    public boolean validateTrustStructureAndWritabilityWithoutSavedData(MessageDataDto data, @Nullable LoggerHook LOG)
    {
        return createTrustValidator(LOG)
                .validate(this.partitionKey(), data);
    }
    
    public boolean validateTrustStructureAndWritability(MessageDataDto data, @Nullable LoggerHook LOG)
    {
        return createTrustValidator(LOG)
                .withSavedDatas(d.requestContext.currentTransaction().getSavedDataMap(this.partitionKey()))
                .validate(this.partitionKey(), data);
    }

    public boolean validateTrustStructureAndWritabilityIncludingStaging(MessageDataDto data, @Nullable LoggerHook LOG)
    {
        return createTrustValidatorIncludingStaging(LOG)
                .withSavedDatas(d.requestContext.currentTransaction().getSavedDataMap(this.partitionKey()))
                .validate(this.partitionKey(), data);
    }
    
    private boolean processData(MessageDataDto data, MessageMetaDto meta, boolean invokeCallbacks, @Nullable LoggerHook LOG) throws IOException, InvalidCipherTextException
    {
        if (d.bootstrapConfig.isExtraValidation()) {
            if (d.validationUtil.validateOrLog(data, LOG) == false) return false;
        }

        MessageDataHeaderDto header = data.getHeader();
        MessageDataDigestDto digest = data.getDigest();
        
        // Extract the header and digest
        if (header == null || digest == null)
        {
            drop(LOG, data, meta, "missing header or digest", null);
            return false;
        }

        // Process it
        return this.promoteChainEntry(new MessageDataMetaDto(data, meta), invokeCallbacks, LOG);
    }


    public <T extends BaseDao> List<UUID> getAllDataIds(Class<T> clazz) {
        String clazzName = clazz.getName();

        List<UUID> ids = new ArrayList<>();
        this.byClazz.computeIfPresent(clazzName, (a, b) -> {
            ids.addAll(b);
            return b;
        });
        return ids;
    }
    
    public <T extends BaseDao> List<DataContainer> getAllData(Class<T> clazz) {
        List<DataContainer> ret = new ArrayList<>();
        for (UUID id : getAllDataIds(clazz)) {
            DataContainer container = this.chainOfTrust.getOrDefault(id, null);
            if (container != null) ret.add(container);
        }
        ret.sort(Comparator.comparing(DataContainer::getFirstOffset));
        return ret;
    }

    public List<DataContainer> getAllData()
    {
        List<DataContainer> ret = new ArrayList<>();
        this.chainOfTrust.forEach( (key, a) -> ret.add(a));
        ret.sort(Comparator.comparing(DataContainer::getFirstOffset));
        return ret;
    }
    
    public boolean exists(UUID id)
    {
        DataContainer container = this.getData(id);
        if (container == null) return false;
        return container.hasPayload();
    }
    
    public boolean everExisted(UUID id)
    {
        DataContainer container = this.getData(id);
        if (container == null) return false;
        return true;
    }
    
    public boolean immutable(UUID id)
    {
        DataContainer container = this.getData(id);
        if (container == null) return false;
        return container.getImmutable();
    }

    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible"})       // We want to return a null if the data does not exist and it must be atomic
    public @Nullable DataContainer getData(UUID id)
    {
        return this.chainOfTrust.getOrDefault(id, null);
    }

    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible"})       // We want to return a null if the data does not exist and it must be atomic
    public @Nullable MessageDataHeaderDto getRootOfTrust(UUID id)
    {
        return rootOfTrust.getOrDefault(id, null);
    }
    
    public Iterable<MessageMetaDto> getHistory(UUID id) {
        DataContainer container = this.getData(id);
        if (container == null) return Collections.emptyList();
        return container.getHistory();
    }
    
    private boolean processCastle(MessageSecurityCastleDto msg, @Nullable LoggerHook LOG) {
        if (d.bootstrapConfig.isExtraValidation()) {
            if (d.validationUtil.validateOrLog(msg, LOG) == false) return false;
        }

        UUID id = msg.getId();
        if (id == null) {
            drop(LOG, msg, null, "missing id", null);
            return false;
        }

        addTrustCastle(msg, LOG);
        return true;
    }

    private boolean processSync(MessageSyncDto msg, @Nullable LoggerHook LOG)
    {
        // Process the message in the sync manager
        d.partitionSyncManager.processSync(msg);

        // All sync messages are instantly tomb-stoned
        this.maintenanceState.tombstone(MessageSerializer.getKey(msg));
        return true;
    }

    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible"})       // We want to return a null if the data does not exist and it must be atomic
    public @Nullable MessageSecurityCastleDto getCastle(UUID id) {
        return castles.getOrDefault(id, null);
    }
    
    private boolean processPublicKey(MessagePublicKeyDto msg, @Nullable LoggerHook LOG) {
        if (d.bootstrapConfig.isExtraValidation()) {
            if (d.validationUtil.validateOrLog(msg, LOG) == false) return false;
        }

        // Now add it to the cache
        publicKeys.put(MessageSerializer.getKey(msg), msg);
        return true;
    }

    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible"})       // We want to return a null if the data does not exist and it must be atomic
    public @Nullable MessagePublicKeyDto getPublicKey(String publicKeyHash) {
        return publicKeys.getOrDefault(publicKeyHash, null);
    }

    public boolean hasPublicKey(@Nullable String _publicKeyHash) {
        @Hash String publicKeyHash = _publicKeyHash;
        if (publicKeyHash == null) return false;
        return publicKeys.containsKey(publicKeyHash);
    }
}