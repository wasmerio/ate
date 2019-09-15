package com.tokera.ate.io.kafka;

import com.tokera.ate.delegates.AteDelegate;
import kafka.network.RequestChannel;
import kafka.security.auth.*;
import org.apache.kafka.common.acl.AclOperation;
import org.apache.kafka.common.security.auth.KafkaPrincipal;

import java.util.Map;

/**
 * Delegate used to check authorization rights in the currentRights context and scopes
 */
public class KafkaAuthorizer implements Authorizer {
    private AteDelegate d = AteDelegate.get();

    private class AuthorityAssessment
    {
        public final RequestChannel.Session session;
        public final AclOperation operation;
        public final Resource resource;
        public final org.apache.kafka.common.resource.ResourceType resourceType;

        public AuthorityAssessment(RequestChannel.Session session, Operation operation, Resource resource) {
            this.session = session;
            this.operation = operation.toJava();
            this.resource = resource;
            this.resourceType = resource.resourceType().toJava();
        }

        public boolean isApi() {
            return true;
        }

        public boolean isTrustedApi() {
            return true;
        }

        public boolean isRead() {
            return this.operation == AclOperation.READ ||
                   this.operation == AclOperation.DESCRIBE;
        }

        public boolean isTrustedAdmin() {
            switch (this.resourceType) {
                case CLUSTER: {
                    if (this.operation == AclOperation.CLUSTER_ACTION) return true;
                    break;
                }
            }

            return false;
        }

        public boolean isNormalAdmin() {
            switch (this.resourceType) {
                case TOPIC: {
                    if (this.operation == AclOperation.WRITE) return true;
                    break;
                }
            }

            return false;
        }

        public boolean isAllowed() {
            if (isRead() == true) return true;
            if (isApi()) {
                if (isNormalAdmin()) return true;
            }
            if (isTrustedApi()) {
                if (isTrustedAdmin()) return true;
            }
            return false;
        }
    }

    @Override
    public boolean authorize(RequestChannel.Session session, Operation operation, Resource resource) {
        AuthorityAssessment assessment = new AuthorityAssessment(session, operation, resource);
        boolean result = assessment.isAllowed();
        d.debugLogging.logKafkaAuthorize(session, operation, resource, result);
        return result;
    }

    @Override
    public void addAcls(scala.collection.immutable.Set<Acl> acls, Resource resource) {

    }

    @Override
    public boolean removeAcls(scala.collection.immutable.Set<Acl> acls, Resource resource) {
        return true;
    }

    @Override
    public boolean removeAcls(Resource resource) {
        return true;
    }

    @Override
    public scala.collection.immutable.Set<Acl> getAcls(Resource resource) {
        return null;
    }

    @Override
    public scala.collection.immutable.Map<Resource, scala.collection.immutable.Set<Acl>> getAcls(KafkaPrincipal principal) {
        return null;
    }

    @Override
    public scala.collection.immutable.Map<Resource, scala.collection.immutable.Set<Acl>> getAcls() {
        return null;
    }

    @Override
    public void close() {
    }

    @Override
    public void configure(Map<String, ?> configs) {
    }
}
