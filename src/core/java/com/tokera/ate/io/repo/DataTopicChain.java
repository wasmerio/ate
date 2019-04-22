package com.tokera.ate.io.repo;

import com.tokera.ate.common.ConcurrentQueue;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.kafka.MessageSerializer;
import com.tokera.ate.dao.msg.*;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.delegates.YamlDelegate;
import com.tokera.ate.security.Encryptor;
import com.google.common.collect.Multimap;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentMap;

import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;
import org.bouncycastle.util.Arrays;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.spongycastle.crypto.InvalidCipherTextException;

/**
 * Represents a cryptographic verified graph of strongly typed data objects that form a chain-of-trust. These chains
 * are effectively the heart of the database.
 */
public class DataTopicChain {
    
    private final String topic;
    private final ConcurrentMap<UUID, MessageDataHeaderDto> rootOfTrust;
    private final ConcurrentMap<UUID, ConcurrentQueue<MessageDataMetaDto>> chainOfPartialTrust;
    private final ConcurrentMap<UUID, DataContainer> chainOfTrust;
    private final ConcurrentMap<String, MessagePublicKeyDto> publicKeys;
    private final ConcurrentMap<String, MessageEncryptTextDto> encryptText;
    private final Multimap<String, String> allowedParents;
    private final Set<String> allowedParentFree;
    private final Encryptor encryptor;
    
    public DataTopicChain(String topic, Multimap<String, String> allowedParents, Set<String> allowedParentFree) {
        this.topic = topic;
        this.rootOfTrust = new ConcurrentHashMap<>();
        this.chainOfTrust = new ConcurrentHashMap<>();
        this.chainOfPartialTrust = new ConcurrentHashMap<>();
        this.publicKeys = new ConcurrentHashMap<>();
        this.encryptText = new ConcurrentHashMap<>();
        this.encryptor = Encryptor.getInstance();
        
        this.allowedParents = allowedParents;
        this.allowedParentFree = allowedParentFree;
    }
    
    public void addTrustDataHeader(MessageDataHeaderDto trustedHeader, @Nullable LoggerHook LOG) {

        MessageDataDto data = new MessageDataDto(
                trustedHeader,
                null,
                null);
        
        if (DataRepoConfig.g_EnableLogging == true) {
            String info = "trust: [->" + this.topic + "]\n" + YamlDelegate.getInstance().serializeObj(trustedHeader);
            if (LOG != null) LOG.info(info); else new LoggerHook(DataTopicChain.class).info(info);
        }

        MessageMetaDto meta = new MessageMetaDto(
                0L,
                0L,
                new Date().getTime());

        this.addTrustData(data, meta, LOG);
    }
    
    public void addTrustKey(MessagePublicKeyDto trustedKey, @Nullable LoggerHook LOG) {
        if (DataRepoConfig.g_EnableLogging == true) {
            String info = "trust: [->" + this.topic + "]\n" + YamlDelegate.getInstance().serializeObj(trustedKey);
            if (LOG != null) LOG.info(info); else new LoggerHook(DataTopicChain.class).info(info);
        }

        @Hash String trustedKeyHash = trustedKey.getPublicKeyHash();
        if (trustedKeyHash != null) {
            this.publicKeys.put(trustedKeyHash, trustedKey);
        }
    }

    @SuppressWarnings({"known.nonnull"})
    public void addTrustData(MessageDataDto data, MessageMetaDto meta, @Nullable LoggerHook LOG) {
        UUID id = data.getHeader().getIdOrThrow();
        this.chainOfTrust.compute(id, (i, c) -> {
            if (c == null) c = new DataContainer();
            return c.add(data, meta);
        });
    }
    
    public void addTrustEncryptText(MessageEncryptTextDto data, @Nullable LoggerHook LOG) {
        this.encryptText.put(MessageSerializer.getKey(data), data);
    }
    
    public String getTopicName() {
        return this.topic;
    }
    
    public boolean rcv(MessageBase raw, MessageMetaDto meta, @Nullable LoggerHook LOG) throws IOException, InvalidCipherTextException {
        
        MessageBaseDto msg;
        switch (raw.msgType()) {
            case MessageType.MessageData:
                msg = new MessageDataDto(raw);
                break;
            case MessageType.MessageEncryptText:
                msg = new MessageEncryptTextDto(raw);
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
        
        if (DataRepoConfig.g_EnableLogging == true) {
            new LoggerHook(DataTopicChain.class).info("rcv:\n" + YamlDelegate.getInstance().serializeObj(msg));
        }
        
        if (msg instanceof MessageDataDto) {
            return processData((MessageDataDto)msg, meta, LOG);
        }
        if (msg instanceof MessagePublicKeyDto) {
            return processPublicKey((MessagePublicKeyDto)msg, LOG);
        }
        if (msg instanceof MessageEncryptTextDto) {
            return processEncryptText((MessageEncryptTextDto)msg, LOG);
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
            index = "topic=" + this.topic + ", offset=" + meta.getOffset();
        } else {
            index = "topic=" + this.topic;
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
            new LoggerHook(DataTopicChain.class).warn(err);
        }
    }
    
    public void drop(@Nullable LoggerHook LOG, MessageDataDto data, String why, @Nullable MessageDataHeader parentHeader) {
        String err;
        
        MessageDataHeaderDto header = data.getHeader();
        UUID id = header.getIdOrThrow();
        err = "Dropping data on [" + this.topic + "] - " + why + " [clazz=" + header.getPayloadClazzOrThrow() + ", id=" + id + "]";

        if (LOG != null) {
            LOG.error(err);
        } else {
            new LoggerHook(DataTopicChain.class).warn(err);
        }
    }

    @SuppressWarnings({"return.type.incompatible"})
    private void reconcileChainEntry(UUID id) {
        chainOfPartialTrust.computeIfPresent(id, (k, q) -> q.size() > 0 ? q : null);
    }
    
    private void promoteChainEntry(UUID id, @Nullable LoggerHook LOG)
    {
        ConcurrentQueue<MessageDataMetaDto> queue = MapTools.getOrNull(chainOfPartialTrust, id);
        if (queue == null) return;
        queue.pollAndConsumeAll((m, l) -> promoteChainEntry(m, l), LOG);
        reconcileChainEntry(id);
    }
    
    public boolean promoteChainEntry(MessageDataMetaDto msg, @Nullable LoggerHook LOG) {
        MessageDataDto data = msg.getData();

        // Validate the data
        if (validateData(data, LOG) == false) {
            return false;
        }
        
        // Add it to the trust tree and return success
        addTrustData(data, msg.getMeta(), LOG);
        return true;
    }
    
    public boolean validate(MessageBaseDto msg, @Nullable LoggerHook LOG)
    {
        if (msg instanceof MessageDataDto) {
            return validateData((MessageDataDto)msg, LOG);
        } else {
            return true;
        }
    }
    
    public boolean validateData(MessageDataDto data, @Nullable LoggerHook LOG)
    {
        return validateData(data, LOG, new HashMap<>());
    }
    
    public boolean validateData(MessageDataDto data, @Nullable LoggerHook LOG, Map<UUID, @Nullable MessageDataDto> requestTrust)
    {
        MessageDataHeaderDto header = data.getHeader();
        MessageDataDigestDto digest = data.getDigest();
        @DaoId UUID id = header.getIdOrThrow();

        // When no digest is attached then this message is not valid
        if (digest == null) return false;
        
        // Make sure its a valid parent we are attached to (or not)
        @DaoId UUID parentId = header.getParentId();
        MessageDataDto parent = null;
        String entityType = header.getPayloadClazzOrThrow();
        if (allowedParents.containsKey(entityType) == false) {
            if (allowedParentFree.contains(entityType) == false) {
                drop(LOG, data, "parent policy not defined for this entity type", null);
                return false;
            }
            if (parentId != null) {
                drop(LOG, data, "parent not allowed for this entity type", null);
                return false;
            }
        } else {
            if (parentId == null) {
                drop(LOG, data, "must have parent for this entity type", null);
                return false;
            }
            
            parent = MapTools.getOrNull(requestTrust, parentId);
            if (parent == null) {
                this.promoteChainEntry(parentId, LOG);

                DataContainer parentMsg = this.getData(parentId, LOG);
                parent = parentMsg != null ? parentMsg.getLastDataOrNull() : null;
            }

            if (parent == null) {
                drop(LOG, data, "parent is missing in chain of trust", null);
                return false;
            } else if (allowedParents.containsEntry(entityType, parent.getHeader().getPayloadClazzOrThrow()) == false) {
                drop(LOG, data, "parent type not allowed [see PermitParentType]", null);
                return false;
            }
        }
        
        // Now make sure this isnt a duplicate object that has suddenly changed
        // parent ownership (as this would violate the chain of trust)
        MessageDataDto existing;
        if (requestTrust.containsKey(id)) {
            existing = requestTrust.get(id);
        } else {
            DataContainer existingMsg = this.getData(id, LOG);
            existing = existingMsg != null ? existingMsg.getLastDataOrNull() : null;
        }
        if (existing != null) {
            @DaoId UUID existingParentId = existing.getHeader().getParentId();
            if (existingParentId != null && existingParentId.equals(header.getParentId()) == false)
            {
                drop(LOG, data, "parent has changed [was=" + existingParentId + ", now=" + header.getParentId() + "]", null);
                return false;
            }
            
            // If the existing header is immutable then fail this update
            if (existing.getHeader().getInheritWrite() == false && existing.getHeader().getAllowWrite().isEmpty()) {
                drop(LOG, data, "record is immutable", null);
                return false;
            }
        }

        // Get the end of the chain of trust that we will traverse up in order
        // to validate the chain of trust. All writes must have a leaf to follow
        // in order to be saved
        MessageDataDto leaf = existing;
        if (leaf == null) leaf = parent;
        if (leaf == null) {
            drop(LOG, data, "record has no leaf to attach to", null);
            return false;
        }
        
        // First check if the digest is allowed to be attached to the parent
        // by doing a role check all the way up the chain until it finds a
        // trusted key hash that matches - later we will check the signature
        // that proves the writer had a copy of this key at the time of writing
        List<String> availableWriteRoles = new ArrayList<>();
        boolean roleFound = false;
        byte[] digestPublicKeyBytes = null;   
        for (;leaf != null;)
        {            
            MessageDataHeaderDto leafHeader = leaf.getHeader();
            Set<String> requiredRoles = leafHeader.getAllowWrite();
            
            for (String trustKeyHash : requiredRoles) {
                availableWriteRoles.add(trustKeyHash);
                if (trustKeyHash.equals(digest.getPublicKeyHash()) == true) {
                    roleFound = true;

                    MessagePublicKeyDto trustPublicKey = this.getPublicKey(trustKeyHash);
                    if (trustPublicKey != null) digestPublicKeyBytes = trustPublicKey.getPublicKeyBytes();
                    if (digestPublicKeyBytes != null) break;
                }
            }
            if (leafHeader.getInheritWrite() == false) break;

            @DaoId UUID leafParentId = leafHeader.getParentId();
            if (leafParentId != null) {

                if (requestTrust.containsKey(leafParentId)) {
                    leaf = requestTrust.get(leafParentId);
                } else {
                    this.promoteChainEntry(leafParentId, LOG);

                    DataContainer leafMsg = this.getData(leafParentId, LOG);
                    leaf = leafMsg != null ? leafMsg.getLastDataOrNull() : null;
                }
            } else {
                leaf = null;
            }
        }
        if (digestPublicKeyBytes == null) {
            MessageDataHeaderDto root = this.getRootOfTrust(id);
            if (root != null) {
                for (String trustKeyHash : root.getAllowWrite()) {
                    availableWriteRoles.add(trustKeyHash);
                    if (trustKeyHash.equals(digest.getPublicKeyHash()) == true) {
                        roleFound = true;

                        MessagePublicKeyDto trustPublicKey = this.getPublicKey(trustKeyHash);
                        if (trustPublicKey != null) digestPublicKeyBytes = trustPublicKey.getPublicKeyBytes();
                        if (digestPublicKeyBytes != null) break;
                    }
                }
            }
        }
        
        if (digestPublicKeyBytes == null || digestPublicKeyBytes.length <= 4) {
            if (roleFound == true) {
                drop(LOG, data, "entity has write roles but public key is missing", null);
            } else {
                String entityTxt = "clazz=" + entityType + ", id=" + id;

                String parentTxt = "null";
                if (parent != null) { parentTxt = "clazz=" + parent.getHeader().getPayloadClazzOrThrow() + ", id=" + parentId; }
                
                StringBuilder sb = new StringBuilder();
                sb.append("entity has no right to attach to its parent");
                sb.append("\n [entity: ").append(entityTxt).append("]");
                sb.append("\n [parent: ").append(parentTxt).append("]");
                for (String role : availableWriteRoles) {
                    sb.append("\n [needs: hash=").append(role);
                    MessagePublicKeyDto roleKey = this.getPublicKey(role);
                    if (roleKey != null && roleKey.getAlias() != null) {
                        sb.append(", alias=").append(roleKey.getAlias());
                    }
                    sb.append("]");
                }

                sb.append("\n [digest: hash=").append(digest.getPublicKeyHash());
                MessagePublicKeyDto digestKey = this.getPublicKey(digest.getPublicKeyHash());
                if (digestKey != null && digestKey.getAlias() != null) {
                    sb.append(", alias=").append(digestKey.getAlias());
                }
                sb.append("]");

                sb.append("\n from ");
                drop(LOG, data, sb.toString(), null);
            }
            return false;
        }
        
        // Compute the byte representation of the header
        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        MessageSerializer.writeBytes(stream, header.createFlatBuffer());
        
        // Add the payload itself into the stream
        if (data.hasPayload()) {
            try {
                byte[] payloadBytes = data.getPayloadBytes();
                if (payloadBytes != null) {
                    stream.write(payloadBytes);
                } else {
                    drop(LOG, data, "message data has payload but it did not appear to be attached", null);
                    return false;
                }
            } catch (IOException ex) {
                String msg = ex.getMessage();
                if (msg == null) msg = ex.toString();
                drop(LOG, data, msg.toLowerCase(), null);
                return false;
            }
        }        
        // Compute the digest bytes
        byte[] streamBytes = stream.toByteArray();
        byte[] seedBytes = digest.getSeedBytes();
        byte[] digestBytes = encryptor.hashSha(seedBytes, streamBytes);

        // Verify the digest bytes match the signature
        byte[] digestBytesHeader = digest.getDigestBytes();
        if (Arrays.areEqual(digestBytesHeader, digestBytes) == false) {
            drop(LOG, data, "digest differential", null);
            return false;
        } 
        
        // Now check that the public yields the same digit thus proving that
        // the owner of the private key generated this data
        byte[] sigBytes = digest.getSignatureBytes();
        
        // Validate that the byte arrays are big enough
        if (digestBytes.length <= 4) {
            drop(LOG, data, "digest of payload bytes invalid", null);
            return false;
        }
        if (sigBytes.length <= 4) {
            drop(LOG, data, "signature bytes invalid", null);
            return false;
        }        
    
        //SLOG.info("ntru-decrypt:\n" + "  - public-key: " + digest.getPublicKey() + "\n  - data: " + digest.getSignature() + "\n");
        if (encryptor.verifyNtru(digestPublicKeyBytes, digestBytes, sigBytes) == false)
        {
            drop(LOG, data, "signature verification failed", null);
            return false;
        }
        
        // Success
        return true;
    }
    
    private boolean processData(MessageDataDto data, MessageMetaDto meta, @Nullable LoggerHook LOG) throws IOException, InvalidCipherTextException
    {
        MessageDataHeaderDto header = data.getHeader();
        MessageDataDigestDto digest = data.getDigest();
        
        // Extract the header and digest
        if (digest == null)
        {
            drop(LOG, data, meta, "missing header or digest", null);
            return false;
        }
        
        // Get the ID and process any existing values that already hold this
        // slot so that the chain of trust grows as new values are added but
        // without causing issues with breaks of the trust chain in the time
        // dimensions
        UUID id = header.getIdOrThrow();
        
        // Construct an index using the validated parameters
        ConcurrentQueue<MessageDataMetaDto> queue = chainOfPartialTrust.computeIfAbsent(id, i -> new ConcurrentQueue<>());
        queue.add(new MessageDataMetaDto(data, meta));
        if (queue.size() > 100) {
            this.promoteChainEntry(id, LOG);
        }
        
        // Success
        return true;
    }
    
    public List<DataContainer> getAllData(@Nullable String _clazz, @Nullable LoggerHook LOG) {
        String clazz = _clazz;

        List<UUID> partialIds = new ArrayList<>();
        this.chainOfPartialTrust.forEach( (key, q) -> {
            MessageDataMetaDto msg = q.peek();
            if (msg == null) return;
            MessageDataDto data = msg.getData();
            if (clazz == null || clazz.equals(data.getHeader().getPayloadClazzOrThrow()) == true) {
                partialIds.add(data.getHeader().getIdOrThrow());
            }
        });
        
        partialIds.forEach( (id) -> {
            this.promoteChainEntry(id, LOG);
        });
        
        List<DataContainer> ret = new ArrayList<>();
        this.chainOfTrust.forEach( (key, a) -> {
            if (clazz == null || clazz.equals(a.getPayloadClazz())) {
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
        this.promoteChainEntry(id, LOG);
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
    
    private boolean processEncryptText(MessageEncryptTextDto msg, @Nullable LoggerHook LOG) {
        encryptText.put(MessageSerializer.getKey(msg), msg);
        return true;
    }

    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible"})       // We want to return a null if the data does not exist and it must be atomic
    public @Nullable MessageEncryptTextDto getEncryptedText(String publicKeyHash, String textHash) {
        String index = publicKeyHash + ":" + textHash;
        return encryptText.getOrDefault(index, null);
    }
    
    private boolean processPublicKey(MessagePublicKeyDto msg, @Nullable LoggerHook LOG)
    {
        // Validate the public key is of sufficient size
        byte[] publicKeyBytes = msg.getPublicKeyBytes();
        if (publicKeyBytes == null) return false;
        if (publicKeyBytes.length <= 64) {
            return false;
        }
        
        // Now add it to the cache
        publicKeys.put(MessageSerializer.getKey(msg), msg);
        return true;
    }

    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible"})       // We want to return a null if the data does not exist and it must be atomic
    public @Nullable MessagePublicKeyDto getPublicKey(String publicKeyHash) {
        return publicKeys.getOrDefault(publicKeyHash, null);
    }
    
    public byte @Nullable [] getPublicKeyBytes(String publicKeyHash) {
        MessagePublicKeyDto ret = this.getPublicKey(publicKeyHash);
        if (ret != null) return ret.getPublicKeyBytes();
        return null;
    }
    
    public boolean hasPublicKey(@Nullable String _publicKeyHash) {
        @Hash String publicKeyHash = _publicKeyHash;
        if (publicKeyHash == null) return false;
        return publicKeys.containsKey(publicKeyHash);
    }
}