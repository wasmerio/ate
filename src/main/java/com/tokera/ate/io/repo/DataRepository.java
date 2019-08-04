package com.tokera.ate.io.repo;

import com.google.common.cache.Cache;
import com.google.common.cache.CacheBuilder;
import com.tokera.ate.dao.IRoles;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;

import javax.annotation.PostConstruct;
import javax.enterprise.context.Dependent;
import javax.enterprise.context.RequestScoped;
import javax.inject.Inject;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.dao.base.BaseDaoInternal;
import com.tokera.ate.dao.enumerations.PermissionPhase;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.io.api.IAteIO;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.security.EffectivePermissionBuilder;
import com.tokera.ate.io.core.StorageSystemFactory;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.dto.EffectivePermissions;
import com.tokera.ate.enumerations.DataPartitionType;

import java.util.*;
import java.util.concurrent.TimeUnit;
import java.util.stream.Collectors;

import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;
import org.checkerframework.checker.nullness.qual.Nullable;

/**
 * Represents a repository of many partition chains that are indexed by partition name
 */
@RequestScoped
public class DataRepository implements IAteIO {

    private AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private StorageSystemFactory factory;
    @SuppressWarnings("initialization.fields.uninitialized")
    private DataSubscriber subscriber;

    public DataRepository() {

    }

    @PostConstruct
    public void init() {
        this.subscriber = factory.get().backend();
        this.LOG.setLogClazz(DataRepository.class);
    }

    private static Cache<String, BaseDao> decryptCache = CacheBuilder.newBuilder()
            .maximumSize(10000)
            .expireAfterWrite(10, TimeUnit.MINUTES)
            .build();

    @Override
    public void warm(IPartitionKey partitionKey) {
        this.subscriber.getPartition(partitionKey, false, DataPartitionType.Dao);
    }

    @Override
    public void warmAndWait(IPartitionKey partitionKey) {
        DataPartition partition = this.subscriber.getPartition(partitionKey, false, DataPartitionType.Dao);
        partition.getBridge().waitTillLoaded();
    }

    @Override
    public @Nullable MessagePublicKeyDto publicKeyOrNull(IPartitionKey partitionKey, @Hash String hash) {
        return publicKeyOrNull(d.requestContext.currentTransaction(), partitionKey, hash);
    }

    public @Nullable MessagePublicKeyDto publicKeyOrNull(DataTransaction trans, IPartitionKey partitionKey, @Hash String hash) {
        DataPartitionChain chain = this.subscriber.getChain(partitionKey);
        MessagePublicKeyDto ret = chain.getPublicKey(hash);
        if (ret != null) return ret;

        ret = trans.findPublicKey(partitionKey, hash);
        if (ret != null) return ret;

        ret = trans.findSavedPublicKey(partitionKey, hash);
        if (ret != null) return ret;

        ret = d.implicitSecurity.findEmbeddedKeyOrNull(hash);
        if (ret != null) return ret;

        return null;
    }

    void validateTrustStructure(BaseDao entity) {

        // Make sure its a valid parent we are attached to
        Class<?> type = entity.getClass();
        String entityType = type.getName();
        @DaoId UUID entityParentId = entity.getParentId();
        if (d.daoParents.getAllowedParentsSimple().containsKey(entityType) == false) {
            if (d.daoParents.getAllowedParentFreeSimple().contains(entityType) == false) {
                if (type.getAnnotation(Dependent.class) == null) {
                    throw new RuntimeException("This entity [" + type.getSimpleName() + "] has not been marked with the Dependent annotation.");
                }
                throw new RuntimeException("This entity [" + type.getSimpleName() + "] has no parent policy defined [see PermitParentType or PermitParentFree annotation].");
            }
            if (entityParentId != null) {
                throw new RuntimeException("This entity [" + type.getSimpleName() + "] is not allowed to be attached to any parents [see PermitParentType annotation].");
            }
        } else {
            if (entityParentId == null) {
                throw new RuntimeException("This entity [" + type.getSimpleName() + "] is not attached to a parent [see PermitParentType annotation].");
            }

            IPartitionKey partitionKey = entity.partitionKey(true);
            DataPartitionChain chain = this.subscriber.getChain(partitionKey);
            DataContainer parentContainer = chain.getData(entityParentId, LOG);
            if (parentContainer != null && d.daoParents.getAllowedParentsSimple().containsEntry(entityType, parentContainer.getPayloadClazz()) == false) {
                if (type.getAnnotation(Dependent.class) == null) {
                    throw new RuntimeException("This entity [" + type.getSimpleName() + "] has not been marked with the Dependent annotation.");
                }
                throw new RuntimeException("This entity is not allowed to be attached to this parent type [see PermitParentEntity annotation].");
            }

            // Make sure the leaf of the chain of trust exists
            String parentClazz;
            if (parentContainer != null) {
                parentClazz = parentContainer.getPayloadClazz();
            } else {
                BaseDao parentEntity = d.requestContext.currentTransaction().find(partitionKey, entityParentId);
                if (parentEntity == null) {
                    throw new RuntimeException("You have yet saved the parent object [" + entity.getParentId() + "] which you must do before this one [" + entity.getId() + "] otherwise the chain of trust will break.");
                }
                parentClazz = BaseDaoInternal.getType(parentEntity);
            }

            // Now make sure the parent type is actually valid
            if (d.daoParents.getAllowedParentsSimple().containsEntry(entityType, parentClazz) == false) {
                StringBuilder sb = new StringBuilder();
                sb.append("This entity is not allowed to be attached to this parent type [see PermitParentEntity annotation].\n");
                for (String allowed : d.daoParents.getAllowedParentsSimple().get(entityType)) {
                    sb.append("  [allowed] -").append(allowed).append("\n");
                }
                if (d.daoParents.getAllowedParentFreeSimple().contains(entityType)) {
                    sb.append("  [allowed] - detached").append("\n");
                }
                throw new RuntimeException(sb.toString());
            }
        }
    }

    String knownPublicKeys(DataTransaction trans, IPartitionKey partitionKey)
    {
        StringBuilder sb = new StringBuilder();
        sb.append("requestPublicKeys:\n");
        for (MessagePublicKeyDto key : trans.findPublicKeys(partitionKey)) {
            sb.append("- ");
            if (key.getAlias() != null) {
                sb.append(key.getAlias()).append(": ");
            }
            sb.append(key.getPublicKeyHash()).append("\n");
        }
        sb.append("requestPrivateKeys:\n");
        for (MessagePrivateKeyDto key : trans.findPrivateKeys(partitionKey)) {
            sb.append("- ");
            if (key.getAlias() != null) {
                sb.append(key.getAlias()).append(": ");
            }
            sb.append(key.getPublicKeyHash()).append("\n");
        }
        sb.append("embeddedPublicKeys:\n");
        for (MessagePublicKeyDto key : d.implicitSecurity.embeddedKeys()) {
            sb.append("- ");
            if (key.getAlias() != null) {
                sb.append(key.getAlias()).append(": ");
            }
            sb.append(key.getPublicKeyHash()).append("\n");
        }
        return sb.toString();
    }

    void validateTrustPublicKeys(DataTransaction trans, BaseDao entity, Collection<String> publicKeys) {
        IPartitionKey partitionKey = entity.partitionKey(true);
        for (String hash : publicKeys) {
            if (hash == null) {
                throw new RuntimeException("Unable to save [" + entity + "] in [" + entity.partitionKey(false) + "] as this object has null public key(s) in one of the role lists.\n" + knownPublicKeys(trans, partitionKey));
            }
            if (this.publicKeyOrNull(trans, partitionKey, hash) == null)
            {
                throw new RuntimeException("Unable to save [" + entity + "] in [" + entity.partitionKey(false) + "] as this object has public key(s) [" + hash + "] that have not yet been saved.\n" + knownPublicKeys(trans, partitionKey));
            }
        }
    }

    void validateTrustPublicKeys(DataTransaction trans, BaseDao entity) {
        if (entity instanceof IRoles) {
            IRoles roles = (IRoles) entity;
            validateTrustPublicKeys(trans, entity, roles.getTrustAllowRead().values());
            validateTrustPublicKeys(trans, entity, roles.getTrustAllowWrite().values());
        }
    }

    void validateReadability(BaseDao entity) {
        EffectivePermissions perms = d.authorization.perms(entity, PermissionPhase.AfterMerge);
        if (perms.rolesRead.size() <= 0) {
            throw d.authorization.buildReadException("Saving this object without any read roles would orphan it, consider deleting it instead.", perms, false);
        }
    }

    void validateWritability(BaseDao entity) {
        EffectivePermissions perms = d.authorization.perms(entity, PermissionPhase.DynamicStaging);
        if (perms.rolesWrite.size() <= 0) {
            throw d.authorization.buildWriteException("Failed to save this object as there are no valid write roles for this spot in the chain-of-trust or its not connected to a parent.", perms, false);
        }
        if (this.immutable(entity.addressableId()) == true) {
            throw new RuntimeException("Unable to save [" + entity + "] as this object is immutable.");
        }
        if (perms.canWrite(d.currentRights) == false) {
            throw d.authorization.buildWriteException(perms, true);
        }
    }

    boolean remove(IPartitionKey partitionKey, BaseDao entity) {

        // Now create and write the data messages themselves
        DataPartition kt = this.subscriber.getPartition(partitionKey);

        if (BaseDaoInternal.hasSaved(entity) == false) {
            return true;
        }

        //String encryptKey64 = d.daoHelper.getEncryptKey(entity, false, this.inMergeDeferred == false, false);
        //if (encryptKey64 == null) return false;

        MessageBaseDto msg = d.dataSerializer.toDataMessage(entity, kt, true);
        kt.write(msg, this.LOG);

        d.debugLogging.logDelete(entity);
        return true;
    }

    boolean remove(PUUID id, Class<?> type) {

        // Loiad the existing record
        DataPartition kt = this.subscriber.getPartition(id.partition());
        DataPartitionChain chain = kt.getChain();
        DataContainer lastContainer = chain.getData(id.id(), LOG);
        if (lastContainer == null) return false;
        if (lastContainer.hasPayload() == false) {
            return true;
        }
        MessageDataHeaderDto lastHeader = lastContainer.getMergedHeader();
        MessageDataHeaderDto header = new MessageDataHeaderDto(lastHeader);

        // Sign the data message
        EffectivePermissions permissions = new EffectivePermissionBuilder(type.getName(), id)
                .withPhase(PermissionPhase.DynamicStaging)
                .build();

        d.requestContext.currentTransaction().put(kt.partitionKey(), d.currentRights.getRightsWrite());

        // Make sure we are actually writing something to Kafka
        MessageDataDigestDto digest = d.dataSignatureBuilder.signDataMessage(id.partition(), header, null, permissions);
        if (digest == null) return false;

        // Clear it from cache and write the data to Kafka
        MessageDataDto data = new MessageDataDto(header, digest, null);
        kt.write(data, this.LOG);

        d.debugLogging.logDelete(id.partition(), data);
        return true;
    }

    @Override
    public boolean exists(@Nullable PUUID _id) {
        PUUID id = _id;
        if (id == null) return false;

        DataPartitionChain kt = this.subscriber.getChain(id.partition());
        if (kt.exists(id.id(), LOG)) return true;
        return false;
    }

    @Override
    public boolean everExisted(@Nullable PUUID _id) {
        PUUID id = _id;
        if (id == null) return false;

        DataPartitionChain chain = this.subscriber.getChain(id.partition());
        if (chain.everExisted(id.id(), LOG)) return true;

        return false;
    }

    @Override
    public boolean immutable(PUUID id) {
        DataPartitionChain chain = this.subscriber.getChain(id.partition());
        if (chain.immutable(id.id(), LOG)) return true;
        return false;
    }

    @Override
    public @Nullable MessageDataHeaderDto readRootOfTrust(PUUID id) {
        DataPartitionChain chain = this.subscriber.getChain(id.partition());
        return chain.getRootOfTrust(id.id());
    }

    @Override
    public @Nullable BaseDao readOrNull(@Nullable PUUID _id, boolean shouldSave) {
        PUUID id = _id;
        if (id == null) return null;

        // Attempt to find the data
        DataPartitionChain chain = this.subscriber.getChain(id.partition());
        DataContainer container = chain.getData(id.id(), LOG);
        if (container == null) return null;

        return container.getMergedData(false, shouldSave);
    }

    @Override
    public BaseDao readOrThrow(PUUID id) {
        DataPartitionChain chain = this.subscriber.getChain(id.partition());
        DataContainer container = chain.getData(id.id(), LOG);
        if (container == null) {
            throw new RuntimeException("Failed to find a data object of id [" + id + "]");
        }

        BaseDao ret = container.getMergedData(true, true);
        if (ret == null) {
            throw new RuntimeException("This object has been removed according to evidence we found that matches data object of id [" + id + "].");
        }
        return ret;
    }

    @Override
    public @Nullable DataContainer readRawOrNull(@Nullable PUUID id) {
        if (id == null) return null;
        DataPartitionChain chain = this.subscriber.getChain(id.partition());
        return chain.getData(id.id(), LOG);
    }

    @Override
    public <T extends BaseDao> Iterable<MessageMetaDto> readHistory(PUUID id, Class<T> clazz) {
        DataPartitionChain chain = this.subscriber.getChain(id.partition());
        return chain.getHistory(id.id(), LOG);
    }

    @Override
    public @Nullable BaseDao readVersionOrNull(PUUID id, MessageMetaDto meta) {
        DataPartition kt = this.subscriber.getPartition(id.partition());

        MessageDataDto data = kt.getBridge().getVersion(id.id(), meta);
        if (data != null) {
            return d.dataSerializer.fromDataMessage(id.partition(), data, false);
        } else {
            this.LOG.warn("missing data [id=" + id + "]");
            return null;
        }
    }

    @Override
    public @Nullable MessageDataDto readVersionMsgOrNull(PUUID id, MessageMetaDto meta) {
        DataPartition kt = this.subscriber.getPartition(id.partition());
        return kt.getBridge().getVersion(id.id(), meta);
    }

    @Override
    public List<BaseDao> readAll(IPartitionKey partitionKey) {
        DataPartitionChain chain = this.subscriber.getChain(partitionKey);

        List<BaseDao> ret = new ArrayList<>();
        for (DataContainer container : chain.getAllData(LOG)) {
            BaseDao entity = container.getMergedData();
            if (entity != null) {
                ret.add(entity);
            }
        }

        return ret;
    }

    @SuppressWarnings({"unchecked"})
    @Override
    public <T extends BaseDao> List<T> readAll(IPartitionKey partitionKey, Class<T> type)
    {
        DataPartitionChain chain = this.subscriber.getChain(partitionKey);

        List<T> ret = new ArrayList<>();
        for (DataContainer container : chain.getAllData(type, LOG)) {
            T entity = (@Nullable T)container.getMergedData();
            if (entity != null) {
                ret.add(entity);
            }
        }
        
        return ret;
    }

    @Override
    public <T extends BaseDao> List<DataContainer> readAllRaw(IPartitionKey partitionKey)
    {
        DataPartitionChain chain = this.subscriber.getChain(partitionKey);
        return chain.getAllData(null, LOG);
    }

    @Override
    public <T extends BaseDao> List<DataContainer> readAllRaw(IPartitionKey partitionKey, @Nullable Class<T> type)
    {
        DataPartitionChain chain = this.subscriber.getChain(partitionKey);

        if (type != null) {
            return chain.getAllData(type, LOG);
        } else {
            return chain.getAllData(null, LOG);
        }
    }

    @Override
    public MessageSyncDto beginSync(IPartitionKey partitionKey, MessageSyncDto sync) {
        return this.subscriber.getPartition(partitionKey).getBridge().startSync(sync);
    }

    @Override
    public boolean finishSync(IPartitionKey partitionKey, MessageSyncDto sync)
    {
        DataPartition kt = this.subscriber.getPartition(partitionKey);
        return kt.getBridge().finishSync(sync);
    }

    @Override
    public DataSubscriber backend() {
        return this.subscriber;
    }

    public void destroyAll() {
        d.ramBridgeBuilder.destroyAll();
        this.subscriber.destroyAll();
    }

    void mergeInternal(DataTransaction trans, IPartitionKey partitionKey, MessageBaseDto data, boolean performSync)
    {
        // Save the data to the bridge and synchronize it
        DataPartition kt = this.subscriber.getPartition(partitionKey);
        IDataPartitionBridge bridge = kt.getBridge();
        bridge.send(data);

        // Synchronize
        if (performSync == true) {
            bridge.sync();
        }

        // If its a public key then we should record that we already saved it
        if (data instanceof MessagePrivateKeyDto) {
            MessagePublicKeyDto key = new MessagePublicKeyDto((MessagePrivateKeyDto) data);
            trans.put(partitionKey, key);
        }
        if (data instanceof MessagePublicKeyDto) {
            MessagePublicKeyDto key = (MessagePublicKeyDto) data;
            trans.put(partitionKey, key);
        }
    }

    private void sendMissingKeys(DataTransaction trans, DataPartition kt)
    {
        sendMissingKeys(trans, kt, trans.findPublicKeys(kt.partitionKey()).stream()
                .map(k -> k.getPublicKeyHash())
                .collect(Collectors.toList()));
        sendMissingKeys(trans, kt, trans.findPrivateKeys(kt.partitionKey()).stream()
                .map(k -> k.getPublicKeyHash())
                .collect(Collectors.toList()));

        for (BaseDao entity : trans.puts(kt.partitionKey())) {
            if (entity instanceof IRoles) {
                IRoles roles = (IRoles) entity;
                sendMissingKeys(trans, kt, roles.getTrustAllowRead().values());
                sendMissingKeys(trans, kt, roles.getTrustAllowWrite().values());
            }
        }
    }

    private void sendMissingKeys(DataTransaction trans, DataPartition kt, Collection<String> roles)
    {
        DataPartitionChain chain = kt.getChain();
        IDataPartitionBridge bridge = kt.getBridge();

        for (String role : roles) {
            if (chain.hasPublicKey(role) == false &&
                trans.findSavedPublicKey(kt.partitionKey(), role) == null)
            {
                MessagePublicKeyDto key = this.publicKeyOrNull(kt.partitionKey(), role);
                if (key == null) {
                    continue;
                }

                bridge.send(key);
                trans.wrote(kt.partitionKey(), key);
            }
        }
    }

    /**
     * Flushes a data transaction to the repository
     * @param trans
     */
    @Override
    public void send(DataTransaction trans, boolean validation) {
        d.debugLogging.logFlush(trans);

        for (IPartitionKey partitionKey : trans.keys().stream().collect(Collectors.toList())) {
            d.debugLogging.logFlush(trans, partitionKey);

            d.requestContext.pushPartitionKey(partitionKey);
            try {
                // Get the partition
                DataPartition kt = this.subscriber.getPartition(partitionKey);
                IDataPartitionBridge bridge = kt.getBridge();

                // Push all the public keys that are in the cache but not known to this partition
                sendMissingKeys(trans, kt);

                // Loop through all the entities and flush them down to the database
                List<MessageDataDto> datas = new ArrayList<>();
                for (BaseDao entity : trans.puts(partitionKey)) {
                    MessageDataDto data = convert(trans, kt, entity);
                    datas.add(data);
                    trans.wrote(partitionKey, data);
                }

                // Write them all out to Kafka
                boolean shouldWait = false;
                for (MessageDataDto data : datas) {
                    bridge.send(data);
                    shouldWait = true;
                }

                // Remove delete any entities that need to be removed
                for (BaseDao entity : trans.deletes(partitionKey)) {
                    remove(partitionKey, entity);
                    shouldWait = true;
                }

                // Cache all the results so they flow between transactions
                for (BaseDao entity : trans.puts(partitionKey)) {
                    trans.cache(partitionKey, entity);
                }

                // Now we wait for the bridge to synchronize
                if (shouldWait) {
                    if (d.currentToken.getWithinTokenScope()) {
                        d.transaction.add(partitionKey, bridge.startSync());
                    } else {
                        bridge.sync();
                    }
                }
            } finally {
                d.requestContext.popPartitionKey();
            }
        }
    }

    private MessageDataDto convert(DataTransaction trans, DataPartition kt, BaseDao entity) {
        DataPartitionChain chain = kt.getChain();

        d.dataRepository.validateTrustPublicKeys(trans, entity);

        MessageDataDto data = (MessageDataDto) d.dataSerializer.toDataMessage(entity, kt, false);

        if (chain.validateTrustStructureAndWritabilityIncludingStaging(data, LOG) == false) {
            String what = "clazz=" + data.getHeader().getPayloadClazzOrThrow() + ", id=" + data.getHeader().getIdOrThrow();
            throw new RuntimeException("The newly created object was not accepted into the chain of trust [" + what + "]");
        }
        d.debugLogging.logMerge(data, entity, false);

        return data;
    }
}
