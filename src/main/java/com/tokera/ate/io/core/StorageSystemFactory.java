package com.tokera.ate.io.core;

import com.tokera.ate.io.IAteIO;
import com.tokera.ate.io.*;
import com.tokera.ate.qualifiers.BackendStorageSystem;
import com.tokera.ate.io.repo.DataRepository;
import com.tokera.ate.io.repo.DataSubscriber;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.inject.Produces;
import javax.enterprise.inject.spi.CDI;

/**
 * Factory used to configure the storage system using a builder and to get a reference the storage
 * system interface itself
 */
@ApplicationScoped
public class StorageSystemFactory
{
    @Nullable
    private Builder tree = null;

    public class Builder
    {
        private IAteIO first;

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
            first = new LayeredIO(
                    CDI.current().select(MemoryCacheIO.class).get(),
                    first
                );
            return this;
        }

        /**
         * @return Reference to an IO interface used to interact with the storage system
         */
        protected IAteIO get() {
            return first;
        }
    }

    /**
     * @return Gets a reference to the IO interface that this factory is configured to create
     */
    @Produces
    @BackendStorageSystem
    public IAteIO get()
    {
        if (tree == null) {
            throw new RuntimeException("You must first initialize this factory by adding a backend and layers.");
        }
        return tree.get();
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
