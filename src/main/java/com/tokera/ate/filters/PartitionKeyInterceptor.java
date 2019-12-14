package com.tokera.ate.filters;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.UUIDTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.providers.PartitionKeySerializer;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import javax.ws.rs.container.ContainerRequestContext;
import javax.ws.rs.container.ContainerRequestFilter;
import javax.ws.rs.ext.Provider;
import javax.annotation.Priority;
import javax.ws.rs.Priorities;
import java.util.UUID;

/**
 * Filter that reads the Topic header from the request and uses this to build a topic scope.
 */
@ApplicationScoped
@Provider
@Priority(Priorities.AUTHORIZATION + 1)
public class PartitionKeyInterceptor implements ContainerRequestFilter {

    protected AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;

    public static final String HEADER_PARTITION_KEY = "PartitionKey";

    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible"})       // We want to return a null if the data does not exist and it must be atomic
    private static @Nullable String getHeaderStringOrNull(ContainerRequestContext requestContext, String s) {
        return requestContext.getHeaderString(s);
    }

    @Override
    public void filter(ContainerRequestContext requestContext)
    {
        // Set the requestContext variable
        d.requestContext.setContainerRequestContext(requestContext);

        // Attempt to extract an ID from the headers (if none exists then we are done)
        String keyText = PartitionKeyInterceptor.getHeaderStringOrNull(requestContext, HEADER_PARTITION_KEY);
        if (keyText == null) return;

        // Enter the partition key scope based on this header
        PartitionKeySerializer serializer = new PartitionKeySerializer();
        IPartitionKey partitionKey = serializer.read(keyText);
        if (partitionKey != null) {
            d.requestContext.pushPartitionKey(partitionKey);
            
            // We warm up the partition locally as it is likely that there will be
            // IO very soon to this repository
            d.io.warm(partitionKey);
        }
    }
}
