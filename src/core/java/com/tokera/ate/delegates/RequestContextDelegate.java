/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.delegates;

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

    @Nullable
    @Context
    private ContainerRequestContext     requestContext;
    private @MonotonicNonNull UriInfo   requestUriInfo;
    private Stack<@TopicName String>    topicNameStack = new Stack<>();

    /**
     * Requests the currentRights container currentRights requestContext that was earlier stored by an filter/interceptor
     * @return Reference to a ContainerRequestContext or null if one was not stored earlier.
     */
    public @Nullable ContainerRequestContext getContainerRequestContextOrNull() {
        return requestContext;
    }

    /**
     * Requests the currentRights container currentRights requestContext that was earlier stored by an filter/interceptor
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
     * Should be called by in the currentRights pipeline at an appropriate point (filter/interceptor) to set the
     * ContainerRequestContext so that it may be used by other components of this engine.
     * @param requestContext Reference to the currentRights requestContext or null if its being cleared
     */
    public void setContainerRequestContext(@Nullable ContainerRequestContext requestContext) {
        this.requestContext = requestContext;
    }

    /**
     * @return Returns true if we are currently in the scope of a particular database topic. If not then the caller
     * can enter a Topic scope using the pushTopicScope method.
     */
    public boolean isWithinTopicScope() {
        return this.topicNameStack.empty() == false;
    }

    /**
     * @return Returns the TopicName for the currentRights topic scope else it throws an exception
     * @throws WebApplicationException Thrown if the caller is not currently in a Topic scope
     */
    public @TopicName String getCurrentTopicScope() {
        try {
            return this.topicNameStack.peek();
        } catch (EmptyStackException ex) {
            throw new WebApplicationException("Request requires a 'Topic' header for this type of currentRights",
                    Response.Status.BAD_REQUEST);
        }
    }

    /**
     * Enters a topic scope and pushes the previous state onto a stack
     * @param topicName Name of the topic to enter
     */
    public void pushTopicScope(@TopicName String topicName) {
        this.topicNameStack.push(topicName);
    }

    /**
     * Restores an earlier pushed topic state from the stack
     * @return Returns the new topic scope that was popped from the stack
     */
    public void popTopicScope() {
        this.topicNameStack.pop();
    }

    /**
     * @return Returns a list of other topics in the stack that are not the currentRights stack
     */
    public Iterable<String> getOtherTopicScopes() {
        String curTopic = this.topicNameStack.peek();
        return this.topicNameStack
                .stream()
                .filter(t -> t.equals(curTopic) == false)
                .collect(Collectors.toList());
    }

    /**
     * Gets a reference to the currentRights URI details for the currentRights that was made
     * @return
     */
    public UriInfo getUriInfo() {
        if (this.requestUriInfo == null) {
            ContainerRequestContext request = this.getContainerRequestContext();
            this.requestUriInfo = request.getUriInfo();
        }
        return this.requestUriInfo;
    }

    /**
     * Gets a reference to the currentRights URI details for the currentRights that was made or null if none exists
     * @return
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
