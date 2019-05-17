package com.tokera.ate.io.repo;

import com.google.common.cache.Cache;
import com.google.common.cache.CacheBuilder;
import com.tokera.ate.dao.base.BaseDao;

import javax.annotation.PostConstruct;
import javax.enterprise.context.RequestScoped;
import javax.inject.Inject;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.io.api.IAteIO;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.api.PartitionKeyComparator;
import com.tokera.ate.security.EffectivePermissionBuilder;
import com.tokera.ate.io.core.StorageSystemFactory;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.delegates.YamlDelegate;
import com.tokera.ate.dto.EffectivePermissions;
import com.tokera.ate.enumerations.DataPartitionType;

import java.util.*;
import java.util.concurrent.TimeUnit;
import java.util.stream.Collectors;

import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;
import org.checkerframework.checker.nullness.qual.Nullable;

/**
 * Represents a repository of many topic chains that are indexed by Topic name
 */
@RequestScoped
public class DataRepository implements IAteIO {

    private AteDelegate d = AteDelegate.getUnsafe();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private StorageSystemFactory factory;
    @SuppressWarnings("initialization.fields.uninitialized")
    private DataSubscriber subscriber;

    private final Map<IPartitionKey, PartitionMergeContext> partitionMergeContexts = new TreeMap<>(new PartitionKeyComparator());

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

    private class PartitionMergeContext {
        public HashSet<UUID> toPutKeys = new HashSet<>();
        public List<BaseDao> toPut = new ArrayList<>();
        public HashSet<UUID> toDeleteKeys = new HashSet<>();
        public List<BaseDao> toDelete = new ArrayList<>();
    }
    
    @Override
    public void warm() {
        IPartitionKey key = d.requestContext.getPartitionKeyScope();
        this.subscriber.getPartition(key, false, DataPartitionType.Dao);
    }
    
    @Override
    public @Nullable MessagePublicKeyDto publicKeyOrNull(@Hash String hash) {
        DataPartitionChain chain = this.subscriber.getChain(d.requestContext.getPartitionKeyScope());
        MessagePublicKeyDto key = chain.getPublicKey(hash);

        if (key == null) {
            for (IPartitionKey topic : d.requestContext.getOtherPartitionKeys()) {
                d.requestContext.pushPartitionKey(topic);
                try {
                    chain = this.subscriber.getChain(topic);
                    key = chain.getPublicKey(hash);
                    if (key != null) break;
                } finally {
                    d.requestContext.popPartitionKey();
                }
            }
        }
        
        return key;
    }
    
    private boolean mergeInternal(BaseDao entity, boolean performValidation, boolean performSync)
    {
        // Get the topic
        DataPartition kt = this.subscriber.getPartition(d.requestContext.getPartitionKeyScope());

        // Generate the data that represents this entity
        DataPartitionChain chain = kt.getChain();
        MessageDataDto data = (MessageDataDto)d.dataSerializer.toDataMessage(entity, kt, false, false);

        // Perform the validations and checks
        if (performValidation && chain.validateData(data, LOG, new HashMap<>()) == false) {
            String what = "clazz=" + data.getHeader().getPayloadClazzOrThrow() + ", id=" + data.getHeader().getIdOrThrow();
            throw new RuntimeException("The newly created object was not accepted into the chain of trust [" + what + "]");
        }

        if (DataRepoConfig.g_EnableLogging == true ||
                DataRepoConfig.g_EnableLoggingWrite == true) {
            this.LOG.info("write: [->" + d.requestContext.getPartitionKeyScope() + "]\n" + YamlDelegate.getInstance().serializeObj(data));
            this.LOG.info("payload: [->" + d.requestContext.getPartitionKeyScope() + "]\n" + YamlDelegate.getInstance().serializeObj(entity));
        }

        // Save the data to the bridge and synchronize it
        IDataPartitionBridge bridge = kt.getBridge();
        bridge.send(data);

        // Synchronize
        if (performSync == true) {
            bridge.sync();
        }

        // Return if the object was actually created
        return exists(entity.getId());
    }

    @Override
    public boolean merge(BaseDao entity) {
        return mergeInternal(entity, true, true);
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

    public void mergeInternal(MessageBaseDto data, boolean performSync)
    {
        // Save the data to the bridge and synchronize it
        DataPartition kt = this.subscriber.getPartition(d.requestContext.getPartitionKeyScope());
        IDataPartitionBridge bridge = kt.getBridge();
        bridge.send(data);

        // Synchronize
        if (performSync == true) {
            bridge.sync();
        }
    }

    @Override
    public boolean merge(MessagePublicKeyDto t) {
        this.mergeInternal(t, true);
        return true;
    }

    @Override
    public boolean merge(MessageEncryptTextDto t) {
        this.mergeInternal(t, true);
        return true;
    }

    private PartitionMergeContext getPartitionMergeContext(IPartitionKey key)
    {
        PartitionMergeContext context;
        if (this.partitionMergeContexts.containsKey(key) == false) {
            context = new PartitionMergeContext();
            this.partitionMergeContexts.put(key, context);
            return context;
        }

        context = this.partitionMergeContexts.get(key);
        assert context != null : "@AssumeAssertion(nullness): The section before ensures that the requestContext can never be null";
        return context;
    }

    private void validateEntityIsChained(BaseDao entity) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();

        // Make sure its a valid parent we are attached to
        String entityType = entity.getClass().getName();
        @DaoId UUID entityParentId = entity.getParentId();
        if (d.daoParents.getAllowedParentsSimple().containsKey(entityType) == false) {
            if (d.daoParents.getAllowedParentFreeSimple().contains(entityType) == false) {
                throw new RuntimeException("This entity [" + entity.getClass().getSimpleName() + "] has no parent policy defined [see PermitParentType or PermitParentFree annotation].");
            }
            if (entityParentId != null) {
                throw new RuntimeException("This entity [" + entity.getClass().getSimpleName() + "] is not allowed to be attached to any parents [see PermitParentType annotation].");
            }
        } else {
            if (entityParentId == null) {
                throw new RuntimeException("This entity [" + entity.getClass().getSimpleName() + "] is not attached to a parent [see PermitParentType annotation].");
            }

            BaseDao parentInCache = this.d.memoryCacheIO.getOrNull(entityParentId);
            DataPartitionChain chain = this.subscriber.getChain(partitionKey);
            DataContainer container = chain.getData(entityParentId, LOG);
            if (container != null && d.daoParents.getAllowedParentsSimple().containsEntry(entityType, container.getPayloadClazz()) == false) {
                throw new RuntimeException("This entity is not allowed to be attached to this parent type [see PermitParentEntity annotation].");
            }

            // Make sure the leaf of the chain of trust exists
            String parentClazz;
            if (container != null) parentClazz = container.getPayloadClazz();
            else if (parentInCache != null) parentClazz = parentInCache.getClass().getName();
            else {
                throw new RuntimeException("You must save the parent object before this one otherwise the chain of trust will break.");
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

    private void validateEntityWritable(BaseDao entity) {
        @DaoId UUID entityParentId = entity.getParentId();
        if (d.authorization.canWrite(entity) == false)
        {
            this.clearDeferred();

            EffectivePermissions permissions = d.authorization.perms(entity.getId(), entityParentId, true);
            throw d.authorization.buildWriteException(entity.getId(), permissions, true);
        }
        if (this.immutable(entity.getId()) == true) {
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
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();

        PartitionMergeContext tContext = this.getPartitionMergeContext(partitionKey);

        if (tContext.toPut.contains(entity.getId()) == true) {
            return;
        }

        // Validate the object is attached to a parent
        if (validate == true) {
            validateEntityIsChained(entity);
        }
        
        // Precache all the encryption toPutKeys ready for the mergeThreeWay deferred phase
        d.daoHelper.getEncryptKey(entity, true, false);
        
        // Validate that we can write to this entity
        if (validate == true) {
            validateEntityWritable(entity);
        }
        
        if (tContext.toPutKeys.contains(entity.getId()) == false) {
            tContext.toPutKeys.add(entity.getId());
            tContext.toPut.add(entity);
        }
        if (tContext.toDeleteKeys.contains(entity.getId()) == true) {
            tContext.toDelete.remove(entity);
            tContext.toDeleteKeys.remove(entity.getId());
        }
    }

    private void mergeDeferredInternal(DataPartition kt, BaseDao entity, List<MessageDataDto> datas, Map<UUID, @Nullable MessageDataDto> requestTrust) {
        DataPartitionChain chain = kt.getChain();

        MessageDataDto data = (MessageDataDto)d.dataSerializer.toDataMessage(entity, kt, false, false);
        datas.add(data);

        if (chain.validateData(data, LOG, requestTrust) == false) {
            String what = "clazz=" + data.getHeader().getPayloadClazzOrThrow() + ", id=" + data.getHeader().getIdOrThrow();
            throw new RuntimeException("The newly created object was not accepted into the chain of trust [" + what + "]");
        }

        if (DataRepoConfig.g_EnableLogging == true || DataRepoConfig.g_EnableLoggingWrite == true) {
            this.LOG.info("write: [->" + d.requestContext.getPartitionKeyScope() + "]\n" + YamlDelegate.getInstance().serializeObj(data));
            this.LOG.info("payload: [->" + d.requestContext.getPartitionKeyScope() + "]\n" + YamlDelegate.getInstance().serializeObj(entity));
        }

        // Add it to the currentRights trust which makes sure that previous
        // records are accounted for during the validation steps
        requestTrust.put(data.getHeader().getIdOrThrow(), data);
    }
    
    @Override
    public void mergeDeferred()
    {
        if (DataRepoConfig.g_EnableLogging == true) {
            this.LOG.info("merge_deferred: [topic_cnt=" + this.partitionMergeContexts.size() + "]\n");
        }

        for (IPartitionKey partitionKey : this.partitionMergeContexts.keySet()) {
            d.requestContext.pushPartitionKey(partitionKey);
            try {
                PartitionMergeContext tContext = this.getPartitionMergeContext(partitionKey);

                // Get the topic
                DataPartition kt = this.subscriber.getPartition(partitionKey);

                // Loop through all the entities and validate them all
                Map<UUID, @Nullable MessageDataDto> requestTrust = new HashMap<>();
                List<MessageDataDto> datas = new ArrayList<>();
                for (BaseDao entity : tContext.toPut.stream().collect(Collectors.toList())) {
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
                for (BaseDao entity : tContext.toDelete) {
                    remove(entity);
                    shouldWait = true;
                }

                // Now we wait for the bridge to synchronize
                if (shouldWait) {
                    if (d.currentToken.getWithinTokenScope()) {
                        d.transaction.add(bridge.startSync());
                    } else {
                        bridge.sync();
                    }
                }
            } finally {
                d.requestContext.popPartitionKey();
            }
        }

        this.partitionMergeContexts.clear();
    }
    
    @Override
    public boolean remove(BaseDao entity) {
        
        // Now create and write the data messages themselves
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        DataPartition kt = this.subscriber.getPartition(partitionKey);

        if (entity.hasSaved() == false) {
            return true;
        }

        //String encryptKey64 = d.daoHelper.getEncryptKey(entity, false, this.inMergeDeferred == false, false);
        //if (encryptKey64 == null) return false;

        MessageBaseDto msg = d.dataSerializer.toDataMessage(entity, kt, true, true);
        kt.write(msg, this.LOG);
        
        if (DataRepoConfig.g_EnableLogging == true) {
            this.LOG.info("remove_payload:\n" + YamlDelegate.getInstance().serializeObj(entity));
        }
        
        return true;
    }
    
    @Override
    public boolean remove(UUID id, Class<?> type) {

        // Loiad the existing record
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        DataPartition kt = this.subscriber.getPartition(partitionKey);
        DataPartitionChain chain = kt.getChain();
        DataContainer lastContainer = chain.getData(id, LOG);
        if (lastContainer == null) return false;
        if (lastContainer.hasPayload() == false) {
            return true;
        }
        MessageDataHeaderDto lastHeader = lastContainer.getMergedHeader();
        MessageDataHeaderDto header = new MessageDataHeaderDto(lastHeader);

        // Sign the data message
        EffectivePermissions permissions = new EffectivePermissionBuilder(d.headIO, id, lastHeader.getParentId())
                .setUsePostMerged(false)
                .build();

        // Make sure we are actually writing something to Kafka
        MessageDataDigestDto digest = d.dataSignatureBuilder.signDataMessage(header, null, permissions);
        if (digest == null) return false;
        
        // Clear it from cache and write the data to Kafka
        MessageDataDto data = new MessageDataDto(header, digest, null);
        kt.write(data, this.LOG);
        if (DataRepoConfig.g_EnableLogging == true) {
            this.LOG.info("remove_payload: " + id);
        }
        
        return true;
    }

    @Override
    public void removeLater(BaseDao entity) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        PartitionMergeContext tContext = this.getPartitionMergeContext(partitionKey);

        // We only actually need to validate and queue if the object has ever been saved
        if (entity.hasSaved() == true)
        {
            // Validate the object is attached to a parent
            validateEntityIsChained(entity);

            // Validate that we can write to this entity
            validateEntityWritable(entity);

            if (tContext.toDeleteKeys.contains(entity.getId()) == false) {
                tContext.toDeleteKeys.add(entity.getId());
                tContext.toDelete.add(entity);
            }
        }

        if (tContext.toPutKeys.contains(entity.getId()) == true) {
            tContext.toPut.remove(entity);
            tContext.toPutKeys.remove(entity.getId());
        }
    }

    @Override
    public void cache(BaseDao entity) {
    }

    @Override
    public void decache(BaseDao entity) {
    }

    @Override
    public boolean exists(@Nullable @DaoId UUID _id) {
        @DaoId UUID id = _id;
        if (id == null) return false;
        
        DataPartitionChain kt = this.subscriber.getChain(d.requestContext.getPartitionKeyScope());
        if (kt.exists(id, LOG)) return true;
        return false;
    }
    
    @Override
    public boolean ethereal() {
        DataPartition topic = this.subscriber.getPartition(d.requestContext.getPartitionKeyScope());
        return topic.ethereal();
    }
    
    @Override
    public boolean everExisted(@Nullable @DaoId UUID _id) {
        @DaoId UUID id = _id;
        if (id == null) return false;
        
        DataPartitionChain chain = this.subscriber.getChain(d.requestContext.getPartitionKeyScope());
        if (chain.everExisted(id, LOG)) return true;
        
        return false;
    }
    
    @Override
    public boolean immutable(UUID id) {
        DataPartitionChain chain = this.subscriber.getChain(d.requestContext.getPartitionKeyScope());
        if (chain.immutable(id, LOG)) return true;
        return false;
    }

    @Override
    public @Nullable MessageDataHeaderDto getRootOfTrust(UUID id) {
        DataPartitionChain chain = this.subscriber.getChain(d.requestContext.getPartitionKeyScope());
        return chain.getRootOfTrust(id);
    }

    @Override
    public @Nullable BaseDao getOrNull(@Nullable @DaoId UUID _id) {
        @DaoId UUID id = _id;
        if (id == null) return null;

        // Attempt to find the data
        DataPartitionChain chain = this.subscriber.getChain(d.requestContext.getPartitionKeyScope());
        DataContainer container = chain.getData(id, LOG);
        if (container == null) return null;

        return container.getMergedData();
    }

    @Override
    public @Nullable DataContainer getRawOrNull(@Nullable UUID id) {
        if (id == null) return null;
        DataPartitionChain chain = this.subscriber.getChain(d.requestContext.getPartitionKeyScope());
        return chain.getData(id, LOG);
    }
    
    @Override
    public <T extends BaseDao> Iterable<MessageMetaDto> getHistory(UUID id, Class<T> clazz) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        DataPartitionChain chain = this.subscriber.getChain(partitionKey);
        return chain.getHistory(id, LOG);
    }
    
    @Override
    public @Nullable BaseDao getVersionOrNull(UUID id, MessageMetaDto meta) {
        DataPartition kt = this.subscriber.getPartition(d.requestContext.getPartitionKeyScope());
        
        MessageDataDto data = kt.getBridge().getVersion(id, meta);
        if (data != null) {
            return d.dataSerializer.fromDataMessage(data, false);
        } else {
            this.LOG.warn("missing data [id=" + id + "]");
            return null;
        }
    }
    
    @Override
    public @Nullable MessageDataDto getVersionMsgOrNull(UUID id, MessageMetaDto meta) {
        DataPartition kt = this.subscriber.getPartition(d.requestContext.getPartitionKeyScope());
        return kt.getBridge().getVersion(id, meta);
    }

    @Override
    public Set<BaseDao> getAll()
    {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
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
    public <T extends BaseDao> Set<T> getAll(Class<T> type)
    {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
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
    public <T extends BaseDao> List<DataContainer> getAllRaw()
    {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        DataPartitionChain chain = this.subscriber.getChain(partitionKey);
        return chain.getAllData(null, LOG);
    }

    @Override
    public <T extends BaseDao> List<DataContainer> getAllRaw(@Nullable Class<T> type)
    {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScope();
        DataPartitionChain chain = this.subscriber.getChain(partitionKey);

        if (type != null) {
            return chain.getAllData(type, LOG);
        } else {
            return chain.getAllData(null, LOG);
        }
    }

    @SuppressWarnings({"unchecked"})
    @Override
    public <T extends BaseDao> List<T> getMany(Collection<UUID> ids, Class<T> type) {
        List<T> ret = new ArrayList<>();
        for (UUID id : ids)
        {
            BaseDao entity = this.getOrNull(id);
            if (entity != null) {
                ret.add((T)entity);
            }
        }
        return ret;
    }

    @Override
    public void clearDeferred() {
        this.partitionMergeContexts.clear();
    }

    @Override
    public void clearCache(@DaoId UUID id) {
    }

    @Override
    public void sync()
    {
        d.transaction.finish();
    }

    @Override
    public boolean sync(MessageSyncDto sync)
    {
        DataPartition kt = this.subscriber.getPartition(d.requestContext.getPartitionKeyScope());
        return kt.getBridge().finishSync(sync);
    }

    @Override
    public DataSubscriber backend() {
        return this.subscriber;
    }
}
