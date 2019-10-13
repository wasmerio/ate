package com.tokera.ate.io.repo;

import com.google.common.cache.Cache;
import com.google.common.cache.CacheBuilder;
import com.google.common.cache.RemovalListener;
import com.google.common.cache.RemovalNotification;
import com.google.common.collect.ImmutableSet;
import com.tokera.ate.dao.IRights;
import com.tokera.ate.dao.IRoles;
import com.tokera.ate.dao.TopicAndPartition;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.base.BaseDaoInternal;
import com.tokera.ate.dao.enumerations.PermissionPhase;
import com.tokera.ate.dao.msg.MessagePrivateKey;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.EffectivePermissions;
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.core.DataPartitionDaemon;
import com.tokera.ate.io.layers.HeadIO;
import com.tokera.ate.io.task.Task;
import com.tokera.ate.scopes.Startup;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.DaoId;
import org.apache.commons.lang3.time.DateUtils;
import org.jboss.weld.context.bound.BoundRequestContext;

import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.inject.spi.CDI;
import javax.ws.rs.container.ContainerRequestContext;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentSkipListSet;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.stream.Collectors;

@Startup
@ApplicationScoped
public class DataMaintenance extends DataPartitionDaemon {
    private final ConcurrentHashMap<TopicAndPartition, State> states;
    private final Random rand = new Random();

    public DataMaintenance() {
        this.states = new ConcurrentHashMap<>();
    }

    @Override
    public void removePartition(TopicAndPartition partition) {
        super.removePartition(partition);
        states.remove(partition);
    }

    public class State implements IRights {
        protected final UUID id;
        protected final IPartitionKey key;
        protected final TopicAndPartition what;
        protected final ConcurrentSkipListSet<String> tombstones;
        protected final Cache<String, Boolean> pendingTombstones;
        protected final ConcurrentSkipListSet<UUID> merges;
        protected final Cache<UUID, Boolean> pendingMerges;
        protected final Cache<String, PrivateKeyWithSeedDto> borrowedReadKeys;
        protected final Cache<String, PrivateKeyWithSeedDto> borrowedWriteKeys;

        protected State(IPartitionKey key) {
            this.id = UUID.randomUUID();
            this.key = key;
            this.what = new TopicAndPartition(key);
            this.tombstones = new ConcurrentSkipListSet<>();
            this.pendingTombstones = CacheBuilder.newBuilder()
                    .expireAfterWrite(d.bootstrapConfig.getDataMaintenanceWindow(), TimeUnit.MILLISECONDS)
                    .removalListener(p -> new RemovalListener<String, Boolean>() {
                        @Override
                        public void onRemoval(RemovalNotification<String, Boolean> n) {
                            if (n.getValue() == Boolean.TRUE) tombstones.add(n.getKey());
                        }
                    })
                    .build();
            this.merges = new ConcurrentSkipListSet<>();
            this.pendingMerges = CacheBuilder.newBuilder()
                    .expireAfterWrite(d.bootstrapConfig.getDataMaintenanceWindow(), TimeUnit.MILLISECONDS)
                    .removalListener(p -> new RemovalListener<UUID, Boolean>() {
                        @Override
                        public void onRemoval(RemovalNotification<UUID, Boolean> n) {
                            if (n.getValue() == Boolean.TRUE) merges.add(n.getKey());
                        }
                    })
                    .build();
            this.borrowedReadKeys = CacheBuilder.newBuilder()
                    .expireAfterWrite(d.bootstrapConfig.getDataMaintenanceWindow() * 2, TimeUnit.MILLISECONDS)
                    .build();
            this.borrowedWriteKeys = CacheBuilder.newBuilder()
                    .expireAfterWrite(d.bootstrapConfig.getDataMaintenanceWindow() * 2, TimeUnit.MILLISECONDS)
                    .build();
        }

        public void tombstone(String key) {
            this.pendingTombstones.put(key, Boolean.TRUE);
        }

        public void dont_tombstone(String key) {
            if (this.pendingTombstones.getIfPresent(key) == Boolean.TRUE) {
                this.pendingTombstones.put(key, Boolean.FALSE);
            }
        }

        public void merge(UUID key) {
            this.pendingMerges.put(key, Boolean.TRUE);
        }

        public void dont_merge(UUID key) {
            if (this.pendingMerges.getIfPresent(key) == Boolean.TRUE) {
                this.pendingMerges.put(key, Boolean.FALSE);
            }
        }

        public void lend_rights(IRights rights) {
            for (PrivateKeyWithSeedDto right : rights.getRightsRead()) {
                this.borrowedReadKeys.put(right.publicHash(), right);
            }
            for (PrivateKeyWithSeedDto right : rights.getRightsWrite()) {
                this.borrowedWriteKeys.put(right.publicHash(), right);
            }
        }

        protected List<String> pollTombstones() {
            List<String> ret = new ArrayList<>();
            for (String key : tombstones) {
                ret.add(key);
            }
            for (String key : ret) {
                tombstones.remove(key);
            }
            return ret;
        }

        protected List<UUID> pollMerges() {
            List<UUID> ret = new ArrayList<>();
            for (UUID key : merges) {
                ret.add(key);
            }
            for (UUID key : ret) {
                merges.remove(key);
            }
            return ret;
        }

        @Override
        public @DaoId UUID getId() {
            return this.id;
        }

        @Override
        public Set<PrivateKeyWithSeedDto> getRightsRead() {
            return this.borrowedReadKeys.asMap().values().stream().collect(Collectors.toSet());
        }

        @Override
        public Set<PrivateKeyWithSeedDto> getRightsWrite() {
            return this.borrowedWriteKeys.asMap().values().stream().collect(Collectors.toSet());
        }

        @Override
        public @Alias String getRightsAlias() {
            return "maintenance:" + key.toString();
        }
    }

    public State getOrCreateState(IPartitionKey key) {
        TopicAndPartition tp = new TopicAndPartition(key);
        return states.computeIfAbsent(tp, k -> new State(key));
    }

    public void lend_rights(IPartitionKey key, IRights rights) {
        State state = getOrCreateState(key);
        state.lend_rights(rights);
    }

    @Override
    protected void work() throws InterruptedException
    {
        // Get the subscriber and compute the next data merging date point
        DataSubscriber subscriber = AteDelegate.get().storageFactory.get().backend();

        // Loop through all the partitions and do the work on them
        for (TopicAndPartition what : listPartitions()) {
            State state = states.getOrDefault(what, null);
            if (state == null) continue;

            // Add a exception handler
            try {
                // Get the bridge and chain
                DataPartition partition = subscriber.getPartition(what, false);
                if (partition == null) continue;
                IDataPartitionBridge bridge = partition.getBridge();
                if (bridge.hasLoaded() == false) continue;

                // Delete anything that should be tomb-stoned
                bridge.deleteMany(state.pollTombstones());

                // Merge everything that needs merging
                List<UUID> toMerge = state.pollMerges();
                if (toMerge.size() > 0)
                {
                    // Create the bounded request context
                    BoundRequestContext boundRequestContext = CDI.current().select(BoundRequestContext.class).get();
                    Task.enterRequestScopeAndInvoke(state.key, boundRequestContext, null, () -> {
                        d.currentRights.impersonate(state);

                        for (UUID id : toMerge) {
                            performMerge(partition, id);
                        }
                    });
                }
            } catch (Throwable ex) {
                if (ex instanceof InterruptedException) throw ex;
                this.LOG.warn(ex);
            }
        }

        // Wait a second (or more) - the random wait time helps reduce merge thrashing
        int waitTime = 1000 + rand.nextInt(4000);
        Thread.sleep(waitTime);
    }

    private void performMerge(DataPartition partition, UUID id)
    {
        IDataPartitionBridge bridge = partition.getBridge();
        if (bridge.hasLoaded() == false) return;
        DataPartitionChain chain = partition.getChain(false);

        // First get the container and check if it still actually needs a merge
        DataContainer container = chain.getData(id);
        if (container.requiresMerge() == false) return;

        // Only if we have the ability to write the object should we attempt to merge it
        if (d.authorization.canWrite(partition.partitionKey(), id)) {
            d.io.write(container.getMergedData());
        }
    }
}
