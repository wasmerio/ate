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
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private DataStagingManager staging;

    public DataRepository() {

    }

    @PostConstruct
    public void init()
    {
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
    public @Nullable MessagePublicKeyDto publicKeyOrNull(IPartitionKey partitionKey, @Hash String hash) {
        MessagePublicKeyDto key = staging.findPublicKey(partitionKey, hash);
        if (key != null) return key;

        DataPartitionChain chain = this.subscriber.getChain(partitionKey);
        return chain.getPublicKey(hash);
    }
    
    private boolean mergeInternal(BaseDao entity, boolean performValidation, boolean performSync)
    {
        // Get the partition
        IPartitionKey key = entity.partitionKey();
        DataPartition kt = this.subscriber.getPartition(key);

        // Validate the public keys
        if (performValidation) {
            validateTrustPublicKeys(entity);
        }

        // Generate the data that represents this entity
        DataPartitionChain chain = kt.getChain();
        MessageDataDto data = (MessageDataDto)d.dataSerializer.toDataMessage(entity, kt, false, false);

        // Perform the validations and checks
        if (performValidation && chain.validateTrustStructureAndWritability(data, LOG, new HashMap<>()) == false) {
            String what = "clazz=" + data.getHeader().getPayloadClazzOrThrow() + ", id=" + data.getHeader().getIdOrThrow();
            throw new RuntimeException("The newly created object was not accepted into the chain of trust [" + what + "]");
        }
        d.debugLogging.logMerge(data, entity, LOG, false);

        // Save the data to the bridge and synchronize it
        IDataPartitionBridge bridge = kt.getBridge();
        bridge.send(data);

        // Synchronize
        if (performSync == true) {
            bridge.sync();
        }

        // Return if the object was actually created
        return exists(entity.addressableId());
    }

    @Override
    public boolean merge(BaseDao entity) {
        return mergeInternal(entity, true, true);
    }

    public void mergeInternal(IPartitionKey partitionKey, MessageBaseDto data, boolean performSync)
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
        if (data instanceof MessagePublicKeyDto) {
            MessagePublicKeyDto key = (MessagePublicKeyDto)data;
            staging.put(partitionKey, key);
        }
    }

    @Override
    public boolean merge(IPartitionKey partitionKey, MessagePublicKeyDto t) {
        this.mergeInternal(partitionKey, t, true);
        return true;
    }

    @Override
    public boolean merge(IPartitionKey partitionKey, MessageSecurityCastleDto t) {
        this.mergeInternal(partitionKey, t, true);
        return true;
    }

    @Override
    public boolean mergeAsync(BaseDao entity) {
        return mergeInternal(entity, true, false);
    }

    @Override
    public boolean mergeWithoutValidation(BaseDao entity) {
        return mergeInternal(entity, false, true);
    }

    @Override
    public boolean mergeAsyncWithoutValidation(BaseDao entity) {
        return mergeInternal(entity, false, false);
    }

    private void validateTrustStructure(BaseDao entity) {

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

            IPartitionKey partitionKey = entity.partitionKey();
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
                BaseDao parentEntity = staging.find(partitionKey, entityParentId);
                if (parentEntity == null) {
                    throw new RuntimeException("You have yet saved the parent object [" + entity.getParentId() + "] which you must do before this one [" + entity.getId() + "] otherwise the chain of trust will break.");
                }
                parentClazz = parentEntity.getClass().getName();
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

    private void validateTrustPublicKeys(BaseDao entity, Collection<String> publicKeys) {
        IPartitionKey partitionKey = entity.partitionKey();
        for (String hash : publicKeys) {
            if (this.publicKeyOrNull(partitionKey, hash) == null) {
                throw new RuntimeException("Unable to save [" + entity + "] as this object has public key(s) [" + hash + "] that have not yet been saved.");
            }
        }
    }

    private void validateTrustPublicKeys(BaseDao entity) {
        if (entity instanceof IRoles) {
            IRoles roles = (IRoles)entity;
            validateTrustPublicKeys(entity, roles.getTrustAllowRead().values());
            validateTrustPublicKeys(entity, roles.getTrustAllowWrite().values());
        }
    }

    private void validateTrustWritability(BaseDao entity) {
        if (d.authorization.canWrite(entity) == false)
        {
            IPartitionKey partitionKey = entity.partitionKey();
            EffectivePermissions permissions = d.authorization.perms(entity);
            throw d.authorization.buildWriteException(partitionKey, entity.getId(), permissions, true);
        }
        if (this.immutable(entity.addressableId()) == true) {
            throw new RuntimeException("Unable to save [" + entity + "] as this object is immutable.");
        }
    }

    @Override
    public void mergeLater(BaseDao t) {
        mergeLaterInternal(t, true);
    }

    @Override
    public void mergeLaterWithoutValidation(BaseDao t) {
        mergeLaterInternal(t, false);
    }

    private void mergeLaterInternal(BaseDao entity, boolean validate) {
        if (validate == true) {
            validateTrustStructure(entity);
            validateTrustPublicKeys(entity);
            validateTrustWritability(entity);
        }

        IPartitionKey partitionKey = entity.partitionKey();
        if (this.staging.find(partitionKey, entity.getId()) != null) {
            return;
        }

        d.debugLogging.logMerge(null, entity, LOG, true);

        this.staging.put(partitionKey, entity);
    }

    private void mergeDeferredInternal(DataPartition kt, BaseDao entity, List<MessageDataDto> datas, Map<UUID, @Nullable MessageDataDto> requestTrust) {
        DataPartitionChain chain = kt.getChain();

        validateTrustPublicKeys(entity);

        MessageDataDto data = (MessageDataDto)d.dataSerializer.toDataMessage(entity, kt, false, false);
        datas.add(data);

        if (chain.validateTrustStructureAndWritabilityIncludingStaging(data, LOG, requestTrust) == false) {
            String what = "clazz=" + data.getHeader().getPayloadClazzOrThrow() + ", id=" + data.getHeader().getIdOrThrow();
            throw new RuntimeException("The newly created object was not accepted into the chain of trust [" + what + "]");
        }
        d.debugLogging.logMerge(data, entity, LOG, false);

        // Add it to the request trust which makes sure that previous
        // records are accounted for during the validation steps
        requestTrust.put(data.getHeader().getIdOrThrow(), data);
    }
    
    @Override
    public void mergeDeferred() {
        d.debugLogging.logMergeDeferred(this.staging, LOG);

        for (IPartitionKey partitionKey : this.staging.keys()) {
            d.requestContext.pushPartitionKey(partitionKey);
            try {
                // Get the partition
                DataPartition kt = this.subscriber.getPartition(partitionKey);

                // Loop through all the entities and validate them all
                Map<UUID, @Nullable MessageDataDto> requestTrust = new HashMap<>();
                List<MessageDataDto> datas = new ArrayList<>();
                for (BaseDao entity : this.staging.puts(partitionKey)) {
                    mergeDeferredInternal(kt, entity, datas, requestTrust);
                }

                // Write them all out to Kafka
                boolean shouldWait = false;
                IDataPartitionBridge bridge = kt.getBridge();
                for (MessageDataDto data : datas) {
                    bridge.send(data);
                    shouldWait = true;
                }

                // Remove delete any entities that need to be removed
                for (BaseDao entity : this.staging.deletes(partitionKey)) {
                    remove(entity);
                    shouldWait = true;
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

        this.staging.clear();
    }
    
    @Override
    public boolean remove(BaseDao entity) {
        
        // Now create and write the data messages themselves
        IPartitionKey partitionKey = entity.partitionKey();
        DataPartition kt = this.subscriber.getPartition(partitionKey);

        if (BaseDaoInternal.hasSaved(entity) == false) {
            return true;
        }

        //String encryptKey64 = d.daoHelper.getEncryptKey(entity, false, this.inMergeDeferred == false, false);
        //if (encryptKey64 == null) return false;

        MessageBaseDto msg = d.dataSerializer.toDataMessage(entity, kt, true, true);
        kt.write(msg, this.LOG);
        
        d.debugLogging.logDelete(entity, LOG);
        return true;
    }
    
    @Override
    public boolean remove(PUUID id, Class<?> type) {

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
        EffectivePermissions permissions = new EffectivePermissionBuilder(id, lastHeader.getParentId())
                .setUsePostMerged(false)
                .build();

        // Make sure we are actually writing something to Kafka
        MessageDataDigestDto digest = d.dataSignatureBuilder.signDataMessage(header, null, permissions);
        if (digest == null) return false;
        
        // Clear it from cache and write the data to Kafka
        MessageDataDto data = new MessageDataDto(header, digest, null);
        kt.write(data, this.LOG);

        d.debugLogging.logDelete(id.partition(), data, LOG);
        return true;
    }

    @Override
    public void removeLater(BaseDao entity) {
        IPartitionKey partitionKey = entity.partitionKey();

        // We only actually need to validate and queue if the object has ever been saved
        if (BaseDaoInternal.hasSaved(entity) == true)
        {
            validateTrustStructure(entity);
            validateTrustPublicKeys(entity);
            validateTrustWritability(entity);

            this.staging.delete(partitionKey, entity);
        }
        else
        {
            this.staging.undo(partitionKey, entity);
        }
    }

    @Override
    public void cache(BaseDao entity) {
    }

    @Override
    public void decache(BaseDao entity) {
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
    public boolean ethereal(IPartitionKey partitionKey) {
        DataPartition partition = this.subscriber.getPartition(partitionKey);
        return partition.ethereal();
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
    public @Nullable MessageDataHeaderDto getRootOfTrust(PUUID id) {
        DataPartitionChain chain = this.subscriber.getChain(id.partition());
        return chain.getRootOfTrust(id.id());
    }

    @Override
    public @Nullable BaseDao getOrNull(@Nullable PUUID _id, boolean shouldSave) {
        PUUID id = _id;
        if (id == null) return null;

        // Attempt to find the data
        DataPartitionChain chain = this.subscriber.getChain(id.partition());
        DataContainer container = chain.getData(id.id(), LOG);
        if (container == null) return null;

        return container.getMergedData(false, shouldSave);
    }

    @Override
    public BaseDao getOrThrow(PUUID id) {
        DataPartitionChain chain = this.subscriber.getChain(id.partition());
        DataContainer container = chain.getData(id.id(), LOG);
        if (container == null) {
            throw new RuntimeException("Failed to find a data object of id [" + id + "]");
        }

        return container.getMergedData(true, true);
    }

    @Override
    public @Nullable DataContainer getRawOrNull(@Nullable PUUID id) {
        if (id == null) return null;
        DataPartitionChain chain = this.subscriber.getChain(id.partition());
        return chain.getData(id.id(), LOG);
    }
    
    @Override
    public <T extends BaseDao> Iterable<MessageMetaDto> getHistory(PUUID id, Class<T> clazz) {
        DataPartitionChain chain = this.subscriber.getChain(id.partition());
        return chain.getHistory(id.id(), LOG);
    }
    
    @Override
    public @Nullable BaseDao getVersionOrNull(PUUID id, MessageMetaDto meta) {
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
    public @Nullable MessageDataDto getVersionMsgOrNull(PUUID id, MessageMetaDto meta) {
        DataPartition kt = this.subscriber.getPartition(id.partition());
        return kt.getBridge().getVersion(id.id(), meta);
    }

    @Override
    public Set<BaseDao> getAll(IPartitionKey partitionKey)
    {
        DataPartitionChain chain = this.subscriber.getChain(partitionKey);

        Set<BaseDao> ret = new HashSet<>();
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
    public <T extends BaseDao> Set<T> getAll(IPartitionKey partitionKey, Class<T> type)
    {
        DataPartitionChain chain = this.subscriber.getChain(partitionKey);

        Set<T> ret = new HashSet<>();
        for (DataContainer container : chain.getAllData(type, LOG)) {
            T entity = (@Nullable T)container.getMergedData();
            if (entity != null) {
                ret.add(entity);
            }
        }
        
        return ret;
    }

    @Override
    public <T extends BaseDao> List<DataContainer> getAllRaw(IPartitionKey partitionKey)
    {
        DataPartitionChain chain = this.subscriber.getChain(partitionKey);
        return chain.getAllData(null, LOG);
    }

    @Override
    public <T extends BaseDao> List<DataContainer> getAllRaw(IPartitionKey partitionKey, @Nullable Class<T> type)
    {
        DataPartitionChain chain = this.subscriber.getChain(partitionKey);

        if (type != null) {
            return chain.getAllData(type, LOG);
        } else {
            return chain.getAllData(null, LOG);
        }
    }

    @SuppressWarnings({"unchecked"})
    @Override
    public <T extends BaseDao> List<T> getMany(IPartitionKey partitionKey, Iterable<UUID> ids, Class<T> type) {
        DataPartitionChain chain = this.subscriber.getChain(partitionKey);

        List<T> ret = new ArrayList<>();
        for (UUID id : ids)
        {
            DataContainer container = chain.getData(id, LOG);
            if (container == null) continue;

            BaseDao entity = container.getMergedData();
            if (entity != null) {
                ret.add((T)entity);
            }
        }
        return ret;
    }

    @Override
    public void clearDeferred() {
        this.staging.clear();
    }

    @Override
    public void clearCache(PUUID id) {
    }

    @Override
    public void sync(IPartitionKey partitionKey)
    {
        this.subscriber.getPartition(partitionKey).getBridge().sync();
    }

    @Override
    public boolean sync(IPartitionKey partitionKey, MessageSyncDto sync)
    {
        DataPartition kt = this.subscriber.getPartition(partitionKey);
        return kt.getBridge().finishSync(sync);
    }

    @Override
    public DataSubscriber backend() {
        return this.subscriber;
    }
}
