package com.tokera.ate.io.repo;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.kafka.MessageSerializer;
import com.tokera.ate.dao.msg.*;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.io.api.IPartitionKey;
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
    private final ConcurrentMap<UUID, MessageDataHeaderDto> rootOfTrust;
    private final ConcurrentMap<UUID, DataContainer> chainOfTrust;
    private final ConcurrentMap<UUID, MessageSecurityCastleDto> castles;
    private final ConcurrentMap<String, MessagePublicKeyDto> publicKeys;
    private final Encryptor encryptor;
    
    public DataPartitionChain(IPartitionKey key) {
        this.key = key;
        this.rootOfTrust = new ConcurrentHashMap<>();
        this.chainOfTrust = new ConcurrentHashMap<>();
        this.publicKeys = new ConcurrentHashMap<>();
        this.castles = new ConcurrentHashMap<>();
        this.encryptor = Encryptor.getInstance();

        this.addTrustKey(d.encryptor.getTrustOfPublicRead(), null);
        this.addTrustKey(d.encryptor.getTrustOfPublicWrite(), null);
    }

    public IPartitionKey partitionKey() { return this.key; }

    public String getPartitionKeyStringValue() {
        return this.key.partitionTopic() + ":" + this.key.partitionIndex();
    }
    
    public void addTrustDataHeader(MessageDataHeaderDto trustedHeader, @Nullable LoggerHook LOG) {

        MessageDataDto data = new MessageDataDto(
                trustedHeader,
                null,
                null);

        d.debugLogging.logTrust(this.partitionKey(), trustedHeader, LOG);

        MessageMetaDto meta = new MessageMetaDto(
                0L,
                0L,
                new Date().getTime());

        this.addTrustData(data, meta, LOG);
    }
    
    public void addTrustKey(MessagePublicKeyDto trustedKey, @Nullable LoggerHook LOG) {
        d.debugLogging.logTrust(this.partitionKey(), trustedKey, LOG);

        if (d.bootstrapConfig.isExtraValidation()) {
            d.validationUtil.validateOrThrow(trustedKey);
        }

        @Hash String trustedKeyHash = trustedKey.getPublicKeyHash();
        if (trustedKeyHash != null) {
            this.publicKeys.put(trustedKeyHash, trustedKey);
        }
    }

    @SuppressWarnings({"known.nonnull"})
    public void addTrustData(MessageDataDto data, MessageMetaDto meta, @Nullable LoggerHook LOG) {
        d.debugLogging.logTrust(this.partitionKey(), data, LOG);

        UUID id = data.getHeader().getIdOrThrow();
        this.chainOfTrust.compute(id, (i, c) -> {
            if (c == null) c = new DataContainer(this.key);
            return c.add(data, meta);
        });
    }
    
    public void addTrustCastle(MessageSecurityCastleDto castle, @Nullable LoggerHook LOG) {
        this.castles.put(castle.getIdOrThrow(), castle);
    }
    
    public boolean rcv(MessageBase raw, MessageMetaDto meta, @Nullable LoggerHook LOG) throws IOException, InvalidCipherTextException {
        
        MessageBaseDto msg;
        switch (raw.msgType()) {
            case MessageType.MessageData:
                msg = new MessageDataDto(raw);
                break;
            case MessageType.MessageSecurityCastle:
                msg = new MessageSecurityCastleDto(raw);
                break;
            case MessageType.MessagePublicKey:
                msg = new MessagePublicKeyDto(raw);
                break;
            default:
                drop(LOG, null, null, "unhandled message type");
                return false;
        }
        
        return rcv(msg, meta, LOG);
    }
    
    public boolean rcv(MessageBaseDto msg, MessageMetaDto meta, @Nullable LoggerHook LOG) throws IOException, InvalidCipherTextException {
        d.debugLogging.logReceive(msg, LOG);

        if (msg instanceof MessageDataDto) {
            return processData((MessageDataDto)msg, meta, LOG);
        }
        if (msg instanceof MessagePublicKeyDto) {
            return processPublicKey((MessagePublicKeyDto)msg, LOG);
        }
        if (msg instanceof MessageSecurityCastleDto) {
            return processCastle((MessageSecurityCastleDto)msg, LOG);
        }
        
        drop(LOG, msg, meta, "unhandled message type");
        return false;
    }
    
    public void drop(@Nullable LoggerHook LOG, @Nullable MessageBaseDto msg, @Nullable MessageMetaDto meta, String why) {
        drop(LOG, msg, meta, why, null);
    }
    
    public void drop(@Nullable LoggerHook LOG, @Nullable MessageBaseDto msg, @Nullable MessageMetaDto meta, String why, @Nullable MessageDataHeader parentHeader) {
        String index;
        if (meta != null) {
            index = "partition=" + this.getPartitionKeyStringValue() + ", offset=" + meta.getOffset();
        } else {
            index = "partition=" + this.getPartitionKeyStringValue();
        }

        String err;
        if (msg instanceof MessageDataDto) {
            MessageDataDto data = (MessageDataDto)msg;
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
    
    public void drop(@Nullable LoggerHook LOG, MessageDataDto data, String why) {
        String err;
        
        MessageDataHeaderDto header = data.getHeader();
        UUID id = header.getIdOrThrow();
        err = "Dropping data on [" + this.getPartitionKeyStringValue() + "] - " + why + " [clazz=" + header.getPayloadClazzOrThrow() + ", id=" + id + "]";

        if (LOG != null) {
            LOG.error(err);
        } else {
            new LoggerHook(DataPartitionChain.class).warn(err);
        }
    }
    
    public boolean promoteChainEntry(MessageDataMetaDto msg, @Nullable LoggerHook LOG) {
        MessageDataDto data = msg.getData();

        // Validate the data
        if (validateTrustStructureAndWritability(data, LOG, null) == false) {
            return false;
        }
        
        // Add it to the trust tree and return success
        addTrustData(data, msg.getMeta(), LOG);
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
    
    public boolean validateTrustStructureAndWritability(MessageDataDto data, @Nullable LoggerHook LOG)
    {
        return validateTrustStructureAndWritability(data, LOG, new HashMap<>());
    }

    public TrustValidatorBuilder createTrustValidator(@Nullable LoggerHook LOG) {
        return new TrustValidatorBuilder()
                .withLogger(LOG)
                .withFailureCallback(f -> this.drop(f.LOG, f.data, f.why))
                .withGetRootOfTrust(id -> this.getRootOfTrust(id))
                .withGetDataCallback(id -> this.getData(id, LOG))
                .withGetPublicKeyCallback(hash -> this.getPublicKey(hash));
    }

    public TrustValidatorBuilder createTrustValidatorIncludingStaging(@Nullable LoggerHook LOG) {
        return createTrustValidator(LOG)
                .withGetPublicKeyCallback(hash -> {
                    MessagePublicKeyDto ret = d.dataStagingManager.findPublicKey(this.key, hash);
                    if (ret != null) return ret;
                    return this.getPublicKey(hash);
                });
    }
    
    public boolean validateTrustStructureAndWritability(MessageDataDto data, @Nullable LoggerHook LOG, @Nullable Map<UUID, @Nullable MessageDataDto> requestTrust)
    {
        if (requestTrust != null) {
            return createTrustValidator(LOG)
                    .withRequestTrust(requestTrust)
                    .validate(this.partitionKey(), data);
        } else {
            return createTrustValidator(LOG)
                    .validate(this.partitionKey(), data);
        }
    }

    public boolean validateTrustStructureAndWritabilityIncludingStaging(MessageDataDto data, @Nullable LoggerHook LOG, Map<UUID, @Nullable MessageDataDto> requestTrust)
    {
        return createTrustValidatorIncludingStaging(LOG)
                .withRequestTrust(requestTrust)
                .validate(this.partitionKey(), data);
    }
    
    private boolean processData(MessageDataDto data, MessageMetaDto meta, @Nullable LoggerHook LOG) throws IOException, InvalidCipherTextException
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
        this.promoteChainEntry(new MessageDataMetaDto(data, meta), LOG);
        return true;
    }
    
    public <T extends BaseDao> List<DataContainer> getAllData(@Nullable Class<T> _clazz, @Nullable LoggerHook LOG) {
        Class<T> clazz = _clazz;
        String clazzName = clazz != null ? clazz.getName() : null;

        List<DataContainer> ret = new ArrayList<>();
        this.chainOfTrust.forEach( (key, a) -> {
            if (clazzName == null || clazzName.equals(a.getPayloadClazz())) {
                ret.add(a);
            }
        });
        return ret;
    }

    public List<DataContainer> getAllData(@Nullable LoggerHook LOG)
    {
        return getAllData(null, LOG);
    }
    
    public boolean exists(UUID id, @Nullable LoggerHook LOG)
    {
        DataContainer container = this.getData(id, LOG);
        if (container == null) return false;
        return container.hasPayload();
    }
    
    public boolean everExisted(UUID id, @Nullable LoggerHook LOG)
    {
        DataContainer container = this.getData(id, LOG);
        if (container == null) return false;
        return true;
    }
    
    public boolean immutable(UUID id, @Nullable LoggerHook LOG)
    {
        DataContainer container = this.getData(id, LOG);
        if (container == null) return false;
        return container.getImmutable();
    }

    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible"})       // We want to return a null if the data does not exist and it must be atomic
    public @Nullable DataContainer getData(UUID id, @Nullable LoggerHook LOG)
    {
        return this.chainOfTrust.getOrDefault(id, null);
    }

    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible"})       // We want to return a null if the data does not exist and it must be atomic
    public @Nullable MessageDataHeaderDto getRootOfTrust(UUID id)
    {
        return rootOfTrust.getOrDefault(id, null);
    }
    
    public Iterable<MessageMetaDto> getHistory(UUID id, @Nullable LoggerHook LOG) {
        DataContainer container = this.getData(id, LOG);
        if (container == null) return new LinkedList<>();
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
        castles.put(msg.getId(), msg);
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