package com.tokera.ate.io.repo;

import com.tokera.ate.dao.*;
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
import com.tokera.ate.io.core.StorageSystemFactory;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.dto.EffectivePermissions;

import java.util.*;
import java.util.function.Predicate;
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
    private Random rand = new Random();

    public DataRepository() {

    }

    @PostConstruct
    public void init() {
        this.subscriber = factory.get().backend();
        this.LOG.setLogClazz(DataRepository.class);
    }

    @Override
    public void warm(IPartitionKey partitionKey) {
        this.subscriber.getOrCreatePartition(partitionKey, false);
    }

    @Override
    public void warmAndWait(IPartitionKey partitionKey) {
        DataPartition partition = this.subscriber.getOrCreatePartition(partitionKey, false);
        partition.getBridge().waitTillLoaded();
    }

    @Override
    public @Nullable MessagePublicKeyDto publicKeyOrNull(IPartitionKey partitionKey, @Hash String hash) {
        return publicKeyOrNull(d.requestContext.currentTransaction(), partitionKey, hash);
    }

    public @Nullable MessagePublicKeyDto publicKeyOrNull(DataTransaction trans, IPartitionKey partitionKey, @Hash String hash) {
        DataPartitionChain chain = this.subscriber.getChain(partitionKey, true);
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
            DataPartitionChain chain = this.subscriber.getChain(partitionKey, true);
            DataContainer parentContainer = chain.getData(entityParentId);
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

    void validateTrustPublicKeys(DataTransaction trans, BaseDao entity, Map<String, String> publicKeys) {
        IPartitionKey partitionKey = entity.partitionKey(true);
        for (Map.Entry<String, String> pair : publicKeys.entrySet()) {
            String hash = pair.getValue();
            if (hash == null) {
                throw new RuntimeException("Unable to save [" + entity + "] in [" + entity.partitionKey(false) + "] as this object has null public key(s) (alias=" + pair.getKey() + ") in one of the role lists.\n" + knownPublicKeys(trans, partitionKey));
            }
            if (this.publicKeyOrNull(trans, partitionKey, hash) == null)
            {
                throw new RuntimeException("Unable to save [" + entity + "] in [" + entity.partitionKey(false) + "] as this object has public key(s) [" + hash + "] (alias=" + pair.getKey() + ") that have not yet been saved.\n" + knownPublicKeys(trans, partitionKey));
            }
        }
    }

    void validateTrustPublicKeys(DataTransaction trans, BaseDao entity) {
        if (entity instanceof IRoles) {
            IRoles roles = (IRoles) entity;
            validateTrustPublicKeys(trans, entity, roles.getTrustAllowRead());
            validateTrustPublicKeys(trans, entity, roles.getTrustAllowWrite());
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
            //DataPartition kt = this.subscriber.getPartition(perms.partitionKey);
            //DataPartitionChain chain = this.subscriber.getChain(perms.partitionKey, true);
            //DataContainer container = chain.getData(perms.id);
            perms = d.authorization.perms(entity, PermissionPhase.DynamicStaging);
            if (perms.rolesWrite.size() <= 0) {
                throw d.authorization.buildWriteException("Failed to save this object as there are no valid write roles for this spot in the chain-of-trust or its not connected to a parent.", perms.rolesWrite, perms, false);
            }
        }
        if (this.immutable(entity.addressableId()) == true) {
            throw new RuntimeException("Unable to save [" + entity + "] as this object is immutable.");
        }
        if (perms.canWrite(d.currentRights) == false) {
            throw d.authorization.buildWriteException(perms.rolesWrite, perms, true);
        }
    }

    boolean remove(IPartitionKey partitionKey, UUID id) {
        DataPartition kt = this.subscriber.getOrCreatePartition(partitionKey);
        DataPartitionChain chain = this.subscriber.getChain(partitionKey, true);
        DataContainer container = chain.getData(id);
        if (container == null) {
            throw new RuntimeException("Failed to find a data object of id [" + id + "]");
        }

        MessageBaseDto msg = d.dataSerializer.toDataMessageDelete(container.getLastHeaderOrNull(), kt);
        kt.write(msg, this.LOG);

        d.debugLogging.logDelete(PUUID.from(partitionKey, id));
        return true;
    }

    @Override
    public boolean exists(@Nullable PUUID _id) {
        PUUID id = _id;
        if (id == null) return false;

        DataPartitionChain kt = this.subscriber.getChain(id.partition(), true);
        if (kt.exists(id.id())) return true;
        return false;
    }

    @Override
    public boolean everExisted(@Nullable PUUID _id) {
        PUUID id = _id;
        if (id == null) return false;

        DataPartitionChain chain = this.subscriber.getChain(id.partition(), true);
        if (chain.everExisted(id.id())) return true;

        return false;
    }

    @Override
    public boolean immutable(PUUID id) {
        DataPartitionChain chain = this.subscriber.getChain(id.partition(), true);
        if (chain.immutable(id.id())) return true;
        return false;
    }

    @Override
    public @Nullable MessageDataHeaderDto readRootOfTrust(PUUID id) {
        DataPartitionChain chain = this.subscriber.getChain(id.partition(), true);
        return chain.getRootOfTrust(id.id());
    }

    @Override
    public @Nullable BaseDao readOrNull(@Nullable PUUID _id) {
        PUUID id = _id;
        if (id == null) return null;

        // Attempt to find the data
        DataPartitionChain chain = this.subscriber.getChain(id.partition(), true);
        DataContainer container = chain.getData(id.id());
        if (container == null) return null;

        return container.fetchData(false);
    }

    @Override
    public BaseDao readOrThrow(PUUID id) {
        DataPartitionChain chain = this.subscriber.getChain(id.partition(), true);
        DataContainer container = chain.getData(id.id());
        if (container == null) {
            throw new RuntimeException("Failed to find a data object of id [" + id + "]");
        }

        BaseDao ret = container.fetchData(true);
        if (ret == null) {
            throw new RuntimeException("This object has been removed according to evidence we found that matches data object of id [" + id + "].");
        }
        return ret;
    }

    @Override
    public @Nullable DataContainer readRawOrNull(@Nullable PUUID id) {
        if (id == null) return null;
        DataPartitionChain chain = this.subscriber.getChain(id.partition(), true);
        return chain.getData(id.id());
    }

    @Override
    public <T extends BaseDao> Iterable<MessageMetaDto> readHistory(PUUID id, Class<T> clazz) {
        DataPartitionChain chain = this.subscriber.getChain(id.partition(), true);
        return chain.getHistory(id.id());
    }

    @Override
    public @Nullable BaseDao readVersionOrNull(PUUID id, long offset) {
        DataPartition kt = this.subscriber.getOrCreatePartition(id.partition());

        MessageDataDto data = kt.getBridge().getVersion(id.id(), offset);
        if (data != null) {
            return d.dataSerializer.fromDataMessage(id.partition(), data, false);
        } else {
            this.LOG.warn("missing data [id=" + id + "]");
            return null;
        }
    }

    @Override
    public @Nullable MessageDataDto readVersionMsgOrNull(PUUID id, long offset) {
        DataPartition kt = this.subscriber.getOrCreatePartition(id.partition());
        return kt.getBridge().getVersion(id.id(), offset);
    }

    @Override
    public List<BaseDao> view(IPartitionKey partitionKey, Predicate<BaseDao> predicate) {
        DataPartitionChain chain = this.subscriber.getChain(partitionKey, true);
        DataTransaction trans = d.requestContext.currentTransaction();

        HashSet<UUID> already = new HashSet<>();
        List<BaseDao> ret = new ArrayList<>();

        for (DataContainer container : chain.getAllData()) {
            EffectivePermissions perms = d.permissionCache.perms(container.getPayloadClazz(), partitionKey, container.id, PermissionPhase.BeforeMerge);
            if (perms.canRead(d.currentRights)) {
                BaseDao entity = container.fetchData();
                if (entity != null) {
                    if (predicate.test(entity)) {
                        ret.add(entity);
                        already.add(container.id);
                    }
                }
            }
        }

        for (BaseDao obj : trans.puts(partitionKey)) {
            if (already.contains(obj.getId())) continue;

            if (predicate.test(obj)) {
                ret.add(obj);
            }
        }

        return ret;
    }

    @SuppressWarnings({"unchecked"})
    @Override
    public <T extends BaseDao> List<T> view(IPartitionKey partitionKey, Class<T> type, Predicate<T> predicate) {
        DataPartitionChain chain = this.subscriber.getChain(partitionKey, true);
        DataTransaction trans = d.requestContext.currentTransaction();

        HashSet<UUID> already = new HashSet<>();
        List<T> ret = new ArrayList<>();

        // Loop through all the data objects that match this type and that have been saved in the past
        for (UUID id : chain.getAllDataIds(type))
        {
            // First search the transaction cache in-case we have an object that we are already
            // working on that was saved to local memory
            BaseDao entity = trans.find(partitionKey, id);
            if (entity != null) {
                if (predicate.test((T)entity)) {
                    ret.add((T) entity);
                    already.add(id);
                    continue;
                }
            }

            // Make sure its not deleted
            if (trans.isDeleted(partitionKey, id)) continue;

            // We should only try and test objects that we actually have rights too
            EffectivePermissions perms = d.permissionCache.perms(type.getName(), partitionKey, id, PermissionPhase.BeforeMerge);
            if (perms.canRead(d.currentRights))
            {
                // We have nothing in the transaction so grab it from the chain-of-trust
                // run the predicate through it (faster than cloning objects) then clone it
                DataContainer container = chain.getData(id);
                if (container == null) continue;
                if (container.test(predicate, false)) {
                    entity = container.fetchData();
                    if (entity != null) {
                        ret.add((T) entity);
                        trans.cache(container.partitionKey, entity);
                        already.add(id);
                        continue;
                    }
                }
            }
        }

        // Finally we need to check for objects that have never been saved to the chain-of-trust
        // but are in the local transaction memory (as these will count)
        for (T obj : trans.putsByType(partitionKey, type)) {
            if (already.contains(obj.getId())) continue;

            if (predicate.test(obj)) {
                ret.add(obj);
            }
        }

        return ret;
    }

    @Override
    public <T extends BaseDao> List<DataContainer> readAllRaw(IPartitionKey partitionKey)
    {
        DataPartitionChain chain = this.subscriber.getChain(partitionKey, true);
        return chain.getAllData();
    }

    @Override
    public <T extends BaseDao> List<DataContainer> readAllRaw(IPartitionKey partitionKey, @Nullable Class<T> type)
    {
        DataPartitionChain chain = this.subscriber.getChain(partitionKey, true);

        if (type != null) {
            return chain.getAllData(type);
        } else {
            return chain.getAllData();
        }
    }

    @Override
    public MessageSyncDto beginSync(IPartitionKey partitionKey, MessageSyncDto sync) {
        DataPartition kt = this.subscriber.getOrCreatePartition(partitionKey);
        MessageSyncDto ret = d.partitionSyncManager.startSync(sync);
        kt.write(ret, this.LOG);
        return ret;
    }

    @Override
    public boolean finishSync(IPartitionKey partitionKey, MessageSyncDto sync)
    {
        return d.partitionSyncManager.finishSync(sync);
    }

    @Override
    public DataSubscriber backend() {
        return this.subscriber;
    }

    public void destroyAll() {
        this.subscriber.destroyAll();
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
        DataPartitionChain chain = kt.getChain(true);
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

        Map<GenericPartitionKey, MessageSyncDto> syncs = new HashMap<>();
        for (IPartitionKey partitionKey : trans.keys().stream().collect(Collectors.toList())) {
            d.debugLogging.logFlush(trans, partitionKey);

            d.dataMaintenance.lend_rights(partitionKey, d.currentRights);

            d.requestContext.pushPartitionKey(partitionKey);
            try {
                // Get the partition
                DataPartition kt = this.subscriber.getOrCreatePartition(partitionKey);
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
                for (UUID entityId : trans.deletes(partitionKey)) {
                    remove(partitionKey, entityId);
                    shouldWait = true;
                }

                // Cache all the results so they flow between transactions
                for (BaseDao entity : trans.puts(partitionKey)) {
                    trans.cache(partitionKey, entity);
                }

                // Now we wait for the bridge to synchronize
                if (shouldWait) {
                    MessageSyncDto sync = new MessageSyncDto(rand.nextLong(), rand.nextLong());
                    if (d.currentToken.getWithinTokenScope()) {
                        d.transaction.add(partitionKey, this.beginSync(partitionKey, sync));
                    } else {
                        syncs.put(new GenericPartitionKey(partitionKey), this.beginSync(partitionKey, sync));
                    }
                }
            } finally {
                d.requestContext.popPartitionKey();
            }
        }

        if (syncs.size() > 0) {
            for (Map.Entry<GenericPartitionKey, MessageSyncDto> e : syncs.entrySet()) {
                this.finishSync(e.getKey(), e.getValue());
            }
        }
    }

    private MessageDataDto convert(DataTransaction trans, DataPartition kt, BaseDao entity) {
        DataPartitionChain chain = kt.getChain(true);

        d.dataRepository.validateTrustPublicKeys(trans, entity);

        MessageDataDto data = (MessageDataDto) d.dataSerializer.toDataMessage(entity, kt);

        if (chain.validateTrustStructureAndWritabilityIncludingStaging(data, LOG) == false) {
            String what = "clazz=" + data.getHeader().getPayloadClazzOrThrow() + ", id=" + data.getHeader().getIdOrThrow();
            throw new RuntimeException("The newly created object was not accepted into the chain of trust [" + what + "]");
        }
        d.debugLogging.logMerge(data, entity, false);

        return data;
    }
}
