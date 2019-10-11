package com.tokera.ate.io.kafka.core;

import com.tokera.ate.BootstrapConfig;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.enumerations.EnquireDomainKeyHandling;
import kafka.network.RequestChannel;
import kafka.security.auth.*;
import org.apache.commons.lang3.time.DateUtils;
import org.apache.kafka.common.acl.AclOperation;
import org.apache.kafka.common.errors.SaslAuthenticationException;
import org.apache.kafka.common.security.JaasContext;
import org.apache.kafka.common.security.auth.KafkaPrincipal;
import org.apache.kafka.common.security.authenticator.SaslServerCallbackHandler;

import javax.security.auth.Subject;
import javax.security.auth.callback.CallbackHandler;
import javax.security.auth.login.AppConfigurationEntry;
import javax.security.auth.login.LoginException;
import javax.security.auth.spi.LoginModule;
import javax.security.sasl.Sasl;
import javax.security.sasl.SaslException;
import javax.security.sasl.SaslServer;
import javax.security.sasl.SaslServerFactory;
import java.io.UnsupportedEncodingException;
import java.security.Provider;
import java.security.Security;
import java.util.*;

/**
 * Delegate used to check authorization rights in the currentRights context and scopes
 */
public class AteKafkaAuthorizer implements Authorizer {
    private AteDelegate d = AteDelegate.get();
    private volatile HashSet<String> allowedClients = new HashSet<>();
    private volatile Date refreshAfter = DateUtils.addYears(new Date(), -10);

    public AteKafkaAuthorizer() {
    }

    private void touch() {
        if (new Date().after(refreshAfter)) {
            refresh();
            refreshAfter = DateUtils.addMinutes(new Date(), 1);
        }
    }

    private void refresh() {
        String bootstrapKafka = BootstrapConfig.propertyOrThrow(d.bootstrapConfig.propertiesForAte(), "kafka.bootstrap");

        HashSet<String> servers = new HashSet<>();
        servers.add("localhost");
        servers.add("127.0.0.1");
        servers.add("::1");
        servers.addAll(d.implicitSecurity.enquireDomainAddresses(bootstrapKafka, EnquireDomainKeyHandling.ThrowOnError));
        this.allowedClients = servers;
    }

    private class AuthorityAssessment
    {
        public final KafkaPrincipal principal;
        public final RequestChannel.Session session;
        public final AclOperation operation;
        public final Resource resource;
        public final org.apache.kafka.common.resource.ResourceType resourceType;
        public final String clientAddress;

        public AuthorityAssessment(RequestChannel.Session session, Operation operation, Resource resource) {
            this.session = session;
            this.operation = operation.toJava();
            this.resource = resource;
            this.resourceType = resource.resourceType().toJava();
            this.principal = session.principal();
            this.clientAddress = this.session.clientAddress().getHostAddress().toLowerCase();
        }

        public boolean isApi() {
            return isTrustedApi();
        }

        public boolean isTrustedApi() {
            if (allowedClients.contains(this.clientAddress)) return true;
            return false;
        }

        public boolean isRead() {
            return this.operation == AclOperation.READ ||
                   this.operation == AclOperation.DESCRIBE;
        }

        public boolean isTrustedAdmin() {
            switch (this.resourceType) {
                case CLUSTER: {
                    if (this.operation == AclOperation.CREATE) return true;
                    if (this.operation == AclOperation.CLUSTER_ACTION) return true;
                    break;
                }
                case TOPIC: {
                    if (this.operation == AclOperation.CREATE) return true;
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
        touch();

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
