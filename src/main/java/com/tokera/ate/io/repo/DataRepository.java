package com.tokera.ate.io.repo;

import com.google.common.cache.Cache;
import com.google.common.cache.CacheBuilder;
import com.tokera.ate.dao.base.BaseDao;

import javax.annotation.PostConstruct;
import javax.enterprise.context.RequestScoped;
import javax.enterprise.inject.spi.CDI;
import javax.inject.Inject;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.io.IAteIO;
import com.tokera.ate.security.EffectivePermissionBuilder;
import com.tokera.ate.io.core.StorageSystemFactory;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.delegates.YamlDelegate;
import com.tokera.ate.dto.EffectivePermissions;
import com.tokera.ate.enumerations.DataTopicType;

import java.util.ArrayList;
import java.util.Collection;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.TimeUnit;
import java.util.stream.Collectors;
import javax.ws.rs.core.Response;

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

    private final Map<String, TopicMergeContext> topicContexts = new HashMap<>();

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

    private class TopicMergeContext {
        public HashSet<UUID> toPutKeys = new HashSet<>();
        public List<BaseDao> toPut = new ArrayList<>();
        public HashSet<UUID> toDeleteKeys = new HashSet<>();
        public List<BaseDao> toDelete = new ArrayList<>();
    }
    
    @Override
    public void warm() {
        String topic = d.requestContext.getCurrentTopicScope();
        this.subscriber.getTopic(topic, false, DataTopicType.Dao);
    }
    
    @Override
    public @Nullable MessagePublicKeyDto publicKeyOrNull(@Hash String hash) {
        DataTopicChain chain = this.subscriber.getChain(d.requestContext.getCurrentTopicScope());
        MessagePublicKeyDto key = chain.getPublicKey(hash);

        if (key == null) {
            for (String topic : d.requestContext.getOtherTopicScopes()) {
                d.requestContext.pushTopicScope(topic);
                try {
                    chain = this.subscriber.getChain(topic);
                    key = chain.getPublicKey(hash);
                    if (key != null) break;
                } finally {
                    d.requestContext.popTopicScope();
                }
            }
        }
        
        return key;
    }
    
    private boolean mergeInternal(BaseDao entity, boolean performValidation, boolean performSync)
    {
        // Get the topic
        DataTopic kt = this.subscriber.getTopic(d.requestContext.getCurrentTopicScope());

        // Generate the data that represents this entity
        DataTopicChain chain = kt.getChain();
        MessageDataDto data = (MessageDataDto)d.dataSerializer.toDataMessage(entity, kt, false, false);

        // Perform the validations and checks
        if (performValidation && chain.validateData(data, LOG, new HashMap<>()) == false) {
            String what = "clazz=" + data.getHeader().getPayloadClazzOrThrow() + ", id=" + data.getHeader().getIdOrThrow();
            throw new RuntimeException("The newly created object was not accepted into the chain of trust [" + what + "]");
        }

        if (DataRepoConfig.g_EnableLogging == true ||
                DataRepoConfig.g_EnableLoggingWrite == true) {
            this.LOG.info("write: [->" + d.requestContext.getCurrentTopicScope() + "]\n" + YamlDelegate.getInstance().serializeObj(data));
            this.LOG.info("payload: [->" + d.requestContext.getCurrentTopicScope() + "]\n" + YamlDelegate.getInstance().serializeObj(entity));
        }

        // Save the data to the bridge and synchronize it
        IDataTopicBridge bridge = kt.getBridge();
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
        DataTopic kt = this.subscriber.getTopic(d.requestContext.getCurrentTopicScope());
        IDataTopicBridge bridge = kt.getBridge();
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

    private TopicMergeContext getPutContext(String topic)
    {
        TopicMergeContext context;
        if (this.topicContexts.containsKey(topic) == false) {
            context = new TopicMergeContext();
            this.topicContexts.put(topic, context);
            return context;
        }

        context = this.topicContexts.get(topic);
        assert context != null : "@AssumeAssertion(nullness): The section before ensures that the requestContext can never be null";
        return context;
    }

    private void validateEntityIsChained(BaseDao entity) {
        String topic = d.requestContext.getCurrentTopicScope();

        // Make sure its a valid parent we are attached to
        String entityType = entity.getClass().getSimpleName();
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
            DataTopicChain chain = this.subscriber.getChain(topic);
            DataContainer container = chain.getData(entityParentId, LOG);
            if (container != null && d.daoParents.getAllowedParentsSimple().containsEntry(entityType, container.getPayloadClazz()) == false) {
                throw new RuntimeException("This entity is not allowed to be attached to this parent type [see PermitParentEntity annotation].");
            }

            // Make sure the leaf of the chain of trust exists
            String parentClazz;
            if (container != null) parentClazz = container.getPayloadClazz();
            else if (parentInCache != null) parentClazz = parentInCache.getClass().getSimpleName();
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
        String topic = d.requestContext.getCurrentTopicScope();

        TopicMergeContext tContext = this.getPutContext(topic);

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

    private void mergeDeferredInternal(DataTopic kt, BaseDao entity, List<MessageDataDto> datas, Map<UUID, @Nullable MessageDataDto> requestTrust) {
        DataTopicChain chain = kt.getChain();

        MessageDataDto data = (MessageDataDto)d.dataSerializer.toDataMessage(entity, kt, false, false);
        datas.add(data);

        if (chain.validateData(data, LOG, requestTrust) == false) {
            String what = "clazz=" + data.getHeader().getPayloadClazzOrThrow() + ", id=" + data.getHeader().getIdOrThrow();
            throw new RuntimeException("The newly created object was not accepted into the chain of trust [" + what + "]");
        }

        if (DataRepoConfig.g_EnableLogging == true || DataRepoConfig.g_EnableLoggingWrite == true) {
            this.LOG.info("write: [->" + d.requestContext.getCurrentTopicScope() + "]\n" + YamlDelegate.getInstance().serializeObj(data));
            this.LOG.info("payload: [->" + d.requestContext.getCurrentTopicScope() + "]\n" + YamlDelegate.getInstance().serializeObj(entity));
        }

        // Add it to the currentRights trust which makes sure that previous
        // records are accounted for during the validation steps
        requestTrust.put(data.getHeader().getIdOrThrow(), data);
    }
    
    @Override
    public void mergeDeferred()
    {
        if (DataRepoConfig.g_EnableLogging == true) {
            this.LOG.info("merge_deferred: [topic_cnt=" + this.topicContexts.size() + "]\n");
        }

        for (String topic : this.topicContexts.keySet()) {
            d.requestContext.pushTopicScope(topic);
            try {
                TopicMergeContext tContext = this.getPutContext(topic);

                // Get the topic
                DataTopic kt = this.subscriber.getTopic(topic);

                // Loop through all the entities and validate them all
                Map<UUID, @Nullable MessageDataDto> requestTrust = new HashMap<>();
                List<MessageDataDto> datas = new ArrayList<>();
                for (BaseDao entity : tContext.toPut.stream().collect(Collectors.toList())) {
                    mergeDeferredInternal(kt, entity, datas, requestTrust);
                }

                // Write them all out to Kafka
                boolean shouldWait = false;
                IDataTopicBridge bridge = kt.getBridge();
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
                d.requestContext.popTopicScope();
            }
        }

        this.topicContexts.clear();
    }
    
    @Override
    public boolean remove(BaseDao entity) {
        
        // Now create and write the data messages themselves
        String topic = d.requestContext.getCurrentTopicScope();
        DataTopic kt = this.subscriber.getTopic(topic);

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
        String topic = d.requestContext.getCurrentTopicScope();
        DataTopic kt = this.subscriber.getTopic(topic);
        DataTopicChain chain = kt.getChain();
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
        String topic = d.requestContext.getCurrentTopicScope();
        TopicMergeContext tContext = this.getPutContext(topic);

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
        
        DataTopicChain kt = this.subscriber.getChain(d.requestContext.getCurrentTopicScope());
        if (kt.exists(id, LOG)) return true;
        return false;
    }
    
    @Override
    public boolean ethereal() {
        DataTopic topic = this.subscriber.getTopic(d.requestContext.getCurrentTopicScope());
        return topic.ethereal();
    }
    
    @Override
    public boolean everExisted(@Nullable @DaoId UUID _id) {
        @DaoId UUID id = _id;
        if (id == null) return false;
        
        DataTopicChain chain = this.subscriber.getChain(d.requestContext.getCurrentTopicScope());
        if (chain.everExisted(id, LOG)) return true;
        
        return false;
    }
    
    @Override
    public boolean immutable(UUID id) {
        DataTopicChain chain = this.subscriber.getChain(d.requestContext.getCurrentTopicScope());
        if (chain.immutable(id, LOG)) return true;
        return false;
    }

    @Override
    public @Nullable MessageDataHeaderDto getRootOfTrust(UUID id) {
        DataTopicChain chain = this.subscriber.getChain(d.requestContext.getCurrentTopicScope());
        return chain.getRootOfTrust(id);
    }

    @Override
    public @Nullable BaseDao getOrNull(@Nullable @DaoId UUID _id) {
        @DaoId UUID id = _id;
        if (id == null) return null;

        // Attempt to find the data
        DataTopicChain chain = this.subscriber.getChain(d.requestContext.getCurrentTopicScope());
        DataContainer container = chain.getData(id, LOG);
        if (container == null) return null;

        return container.getMergedData();
    }

    @Override
    public @Nullable DataContainer getRawOrNull(@Nullable UUID id) {
        if (id == null) return null;
        DataTopicChain chain = this.subscriber.getChain(d.requestContext.getCurrentTopicScope());
        return chain.getData(id, LOG);
    }
    
    @Override
    public <T extends BaseDao> Iterable<MessageMetaDto> getHistory(UUID id, Class<T> clazz) {
        String topic = d.requestContext.getCurrentTopicScope();
        DataTopicChain chain = this.subscriber.getChain(topic);
        return chain.getHistory(id, LOG);
    }
    
    @Override
    public @Nullable BaseDao getVersionOrNull(UUID id, MessageMetaDto meta) {
        DataTopic kt = this.subscriber.getTopic(d.requestContext.getCurrentTopicScope());
        
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
        DataTopic kt = this.subscriber.getTopic(d.requestContext.getCurrentTopicScope());
        return kt.getBridge().getVersion(id, meta);
    }

    @Override
    public Set<BaseDao> getAll()
    {
        String topic = d.requestContext.getCurrentTopicScope();
        DataTopicChain chain = this.subscriber.getChain(topic);

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
        String topic = d.requestContext.getCurrentTopicScope();
        DataTopicChain chain = this.subscriber.getChain(topic);

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
        String topic = d.requestContext.getCurrentTopicScope();
        DataTopicChain chain = this.subscriber.getChain(topic);
        return chain.getAllData(null, LOG);
    }

    @Override
    public <T extends BaseDao> List<DataContainer> getAllRaw(@Nullable Class<T> type)
    {
        String topic = d.requestContext.getCurrentTopicScope();
        DataTopicChain chain = this.subscriber.getChain(topic);

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
        this.topicContexts.clear();
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
        DataTopic kt = this.subscriber.getTopic(d.requestContext.getCurrentTopicScope());
        return kt.getBridge().finishSync(sync);
    }

    @Override
    public DataSubscriber backend() {
        return this.subscriber;
    }
}
