/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.delegates;

import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.units.TopicName;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.RequestScoped;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.container.ContainerRequestContext;
import javax.ws.rs.core.Context;
import javax.ws.rs.core.Response;
import javax.ws.rs.core.UriInfo;
import java.util.*;
import java.util.stream.Collectors;

/**
 * Class used to share some critical fields between filters/interceptors and core engine components.
 * @author John Sharratt (johnathan.sharratt@gmail.com)
 */
@RequestScoped
public class RequestContextDelegate {

    private @Context @Nullable ContainerRequestContext requestContext;
    private @MonotonicNonNull UriInfo requestUriInfo;
    private Stack<@TopicName IPartitionKey> partitionKeyStack = new Stack<>();

    /**
     * Requests the currentRights container currentRights requestContext that was earlier stored by an filter/interceptor
     * @return Reference to a ContainerRequestContext or null if one was not stored earlier.
     */
    public @Nullable ContainerRequestContext getContainerRequestContextOrNull() {
        return requestContext;
    }

    /**
     * Requests the currentRights container current requestContext that was earlier stored by an filter/interceptor
     * @return Reference to a ContainerRequestContext throws an exception if one was not stored earlier
     * @throws WebApplicationException Thrown if the ContainerRequestContext was not stored earlier by the
     * filter/interceptor
     */
    public ContainerRequestContext getContainerRequestContext() {
        ContainerRequestContext ret = requestContext;
        if (ret == null) {
            throw new WebApplicationException("Request requestContext does not exist.", Response.Status.INTERNAL_SERVER_ERROR);
        }
        return ret;
    }

    /**
     * Should be called by in the current pipeline at an appropriate point (filter/interceptor) to set the
     * ContainerRequestContext so that it may be used by other components of this engine.
     * @param requestContext Reference to the currentRights requestContext or null if its being cleared
     */
    public void setContainerRequestContext(@Nullable ContainerRequestContext requestContext) {
        this.requestContext = requestContext;
    }

    /**
     * @return Returns true if we are currently in the scope of a particular database partition. If not then the caller
     * can enter a partition scope using the pushTopicScope method.
     */
    public boolean isWithinPartitionKeyScope() {
        return this.partitionKeyStack.empty() == false;
    }

    /**
     * @return Returns the partition key for the current partition scope else it throws an exception
     * @throws WebApplicationException Thrown if the caller is not currently in a partition scope
     */
    public IPartitionKey getPartitionKeyScope() {
        try {
            return this.partitionKeyStack.peek();
        } catch (EmptyStackException ex) {
            throw new WebApplicationException("Request requires a 'PartitionKey' header for this type of currentRights",
                    Response.Status.BAD_REQUEST);
        }
    }

    /**
     * @return Returns the partition key for the current partition scope else it throws an exception or null if it
     * doesn't exist in the current context
     */
    public @Nullable IPartitionKey getPartitionKeyScopeOrNull() {
        try {
            if (this.partitionKeyStack.empty()) return null;
            return this.partitionKeyStack.peek();
        } catch (EmptyStackException ex) {
            return null;
        }
    }

    /**
     * Enters a partition key scope and pushes the previous key onto a stack
     */
    public void pushPartitionKey(@TopicName IPartitionKey key) {
        this.partitionKeyStack.push(key);
    }

    /**
     * Restores an earlier pushed partition key from the stack
     * @return Returns the new partition key that was popped from the stack
     */
    public IPartitionKey popPartitionKey() {
        return this.partitionKeyStack.pop();
    }

    /**
     * @return Returns a list of other partition keys in the stack that are not the current partition key itself
     */
    public Iterable<IPartitionKey> getOtherPartitionKeys() {
        if (this.partitionKeyStack.empty()) return new LinkedList<>();
        IPartitionKey curKey = this.partitionKeyStack.peek();
        if (curKey == null) return this.partitionKeyStack;
        return this.partitionKeyStack
                .stream()
                .filter(t -> t.equals(curKey) == false)
                .collect(Collectors.toList());
    }

    /**
     * Gets a reference to the current URI details for the currentRights that was made
     */
    public UriInfo getUriInfo() {
        if (this.requestUriInfo == null) {
            ContainerRequestContext request = this.getContainerRequestContext();
            this.requestUriInfo = request.getUriInfo();
        }
        return this.requestUriInfo;
    }

    /**
     * Gets a reference to the current URI details for the currentRights that was made or null if none exists
     */
    public @Nullable UriInfo getUriInfoOrNull() {
        if (this.requestUriInfo == null) {
            ContainerRequestContext request = this.getContainerRequestContextOrNull();
            if (request == null) return null;
            this.requestUriInfo = request.getUriInfo();
        }
        return this.requestUriInfo;
    }
}
