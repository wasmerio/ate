package com.tokera.ate.io.core;

import com.tokera.ate.io.api.*;
import com.tokera.ate.io.layers.AccessLogIO;
import com.tokera.ate.io.layers.BackendIO;
import com.tokera.ate.io.layers.SplitIO;
import com.tokera.ate.io.layers.MemoryRequestCacheIO;
import com.tokera.ate.qualifiers.BackendStorageSystem;
import com.tokera.ate.io.repo.DataRepository;
import com.tokera.ate.io.repo.DataSubscriber;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.inject.Produces;
import javax.enterprise.inject.spi.CDI;

/**
 * Factory used to configure the storage system using a builder and to getData a reference the storage
 * system interface itself
 */
@ApplicationScoped
public class StorageSystemFactory
{
    private @Nullable Builder tree = null;

    public class Builder
    {
        private IAteIO first;
        private IPartitionResolver partitionResolver = new DefaultPartitionResolver();
        private IPartitionKeyMapper partitionKeyMapper = new DefaultPartitionKeyMapper();
        private ISecurityCastleFactory secureKeyRepository = new DefaultSecurityCastleFactory();
        private ITokenParser tokenParser = new DefaultTokenParser();

        protected Builder(IAteIO next) {
            first = next;
        }

        /**
         * Adds a access logger layer that will record all the read and write operations. This layer can be used
         * to invalidate client side caching.
         */
        public Builder addAccessLoggerLayer() {
            first = new AccessLogIO(first);
            return this;
        }

        /**
         * Adds a caching layer to the storage system that will improve performance of multiple requests to the
         * same data objects during the same currentRights
         */
        public Builder addCacheLayer() {
            first = new SplitIO(
                    CDI.current().select(MemoryRequestCacheIO.class).get(),
                    first
                );
            return this;
        }

        /**
         * Overrides the default partition resolver with a custom one
         * @param resolver Reference to a custom partition resolver to use instead of the default one
         */
        public Builder withPartitionResolver(IPartitionResolver resolver) {
            this.partitionResolver = resolver;
            return this;
        }

        /**
         * Overrides the default partition key mapper with a custom one
         * @param mapper Reference to custom partition key mapper implementation instead of the default one
         */
        public Builder withPartitionKeyMapper(IPartitionKeyMapper mapper) {
            this.partitionKeyMapper = mapper;
            return this;
        }

        /**
         * Overrides the default secure key repository with a custom one
         * @param repository Reference to the secure key resolver to use instead of the default one
         */
        public Builder withSecureKeyRepository(ISecurityCastleFactory repository) {
            this.secureKeyRepository = repository;
            return this;
        }

        /**
         * Overrides the default token parser with a custom one
         * @param tokenParser Reference to an implementation of a token parser to use instead of the default one
         */
        public Builder withTokenParser(ITokenParser tokenParser) {
            this.tokenParser = tokenParser;
            return this;
        }
    }

    /**
     * @return Gets a reference to the IO interface that this factory is configured to create
     */
    @Produces
    @BackendStorageSystem
    public IAteIO get() {
        if (tree == null) {
            throw new RuntimeException("You must first initialize this factory by adding a backend and layers.");
        }
        return tree.first;
    }

    /**
     * @return Gets a reference to the interface used to determine the topic and partition for a data object
     */
    @Produces
    @BackendStorageSystem
    public IPartitionResolver partitionResolver() {
        if (tree == null) {
            throw new RuntimeException("You must first initialize this factory by adding a backend and layers.");
        }
        return tree.partitionResolver;
    }

    @Produces
    @BackendStorageSystem
    public IPartitionKeyMapper partitionKeyMapper() {
        if (tree == null) {
            throw new RuntimeException("You must first initialize this factory by adding a backend and layers.");
        }
        return tree.partitionKeyMapper;
    }

    /**
     * @return Gets a reference to the interface used to find the secure encryption keys for a data objects
     */
    @Produces
    @BackendStorageSystem
    public ISecurityCastleFactory secureKeyRepository() {
        if (tree == null) {
            throw new RuntimeException("You must first initialize this factory by adding a backend and layers.");
        }
        return tree.secureKeyRepository;
    }

    /**
     * @return Gets a reference to the interface used for parsing tokens into useful things
     */
    @Produces
    @BackendStorageSystem
    public ITokenParser tokenParser() {
        if (tree == null) {
            throw new RuntimeException("You must first initialize this factory by adding a backend and layers.");
        }
        return tree.tokenParser;
    }

    /**
     * Builds a new storage system based on a Kafka distributed commit log as the backend
     * @return Layer builder that allows one to add more complex storage subsystems
     */
    public Builder buildKafkaBackend() {
        Builder ret = new Builder(
                new BackendIO(
                    CDI.current().select(DataRepository.class).get(),
                    new DataSubscriber(DataSubscriber.Mode.Kafka)
                )
            );
        this.tree = ret;
        return ret;
    }

    /**
     * Builds a new storage system based on a pure RAM implementation
     * @return Layer builder that allows one to add more complex storage subsystems
     */
    public Builder buildRamBackend() {
        Builder ret = new Builder(
                new BackendIO(
                    CDI.current().select(DataRepository.class).get(),
                    new DataSubscriber(DataSubscriber.Mode.Ram)
                )
            );
        this.tree = ret;
        return ret;
    }
}
