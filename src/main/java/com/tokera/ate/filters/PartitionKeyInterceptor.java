package com.tokera.ate.filters;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.UUIDTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.io.api.IPartitionKey;
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
@Priority(Priorities.AUTHENTICATION)
public class PartitionKeyInterceptor implements ContainerRequestFilter {

    protected AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private DefaultBootstrapInit interceptorInit;

    public static final String HEADER_TOPIC = "Topic";
    public static final String HEADER_PARTITION_KEY = "PartitionKey";

    @PostConstruct
    public void init() {
        interceptorInit.touch();
    }

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
        String idTxt = PartitionKeyInterceptor.getHeaderStringOrNull(requestContext, HEADER_PARTITION_KEY);
        if (idTxt == null) idTxt = PartitionKeyInterceptor.getHeaderStringOrNull(requestContext, HEADER_TOPIC);
        if (idTxt == null) return;

        // Convert it to a partition key
        UUID id = UUIDTools.parseUUIDorNull(idTxt);
        IPartitionKey partitionKey = d.headIO.partitionKeyMapper().resolve(id);
        if (id == null) id = UUIDTools.generateUUID(idTxt);

        // Enter the partition key scope based on this header
        if (partitionKey != null) {
            d.requestContext.pushPartitionKey(partitionKey);
            
            // We warm up the partition locally as it is likely that there will be
            // IO very soon to this repository
            d.headIO.warm();
        }
    }
}
