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
import javax.security.auth.login.LoginException;
import javax.security.auth.spi.LoginModule;
import javax.security.sasl.Sasl;
import javax.security.sasl.SaslException;
import javax.security.sasl.SaslServer;
import javax.security.sasl.SaslServerFactory;
import java.io.UnsupportedEncodingException;
import java.security.Provider;
import java.security.Security;
import java.util.Arrays;
import java.util.Date;
import java.util.HashSet;
import java.util.Map;

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

    public static class KafkaLoginModule implements LoginModule {
        private final AteDelegate d = AteDelegate.get();

        private static final String USERNAME_CONFIG = "username";
        private static final String PASSWORD_CONFIG = "password";

        static {
            KafkaSaslServerProvider.initialize();
        }

        @Override
        public void initialize(Subject subject, CallbackHandler callbackHandler, Map<String, ?> sharedState, Map<String, ?> options) {
            String username = (String) options.get(USERNAME_CONFIG);
            if (username != null)
                subject.getPublicCredentials().add(username);
            String password = (String) options.get(PASSWORD_CONFIG);
            if (password != null) {
                subject.getPrivateCredentials().add(password);
            }
        }

        @Override
        public boolean login() throws LoginException {
            return true;
        }

        @Override
        public boolean logout() throws LoginException {
            return true;
        }

        @Override
        public boolean commit() throws LoginException {
            return true;
        }

        @Override
        public boolean abort() throws LoginException {
            return false;
        }
    }

    public static class KafkaSaslServer implements SaslServer {
        public static final String PLAIN_MECHANISM = "PLAIN";
        private static final String JAAS_USER_PREFIX = "user_";

        private final JaasContext jaasContext;

        private boolean complete;
        private String authorizationId;

        public KafkaSaslServer(JaasContext jaasContext) {
            this.jaasContext = jaasContext;
        }

        /**
         * @throws SaslAuthenticationException if username/password combination is invalid or if the requested
         *         authorization id is not the same as username.
         * <p>
         * <b>Note:</b> This method may throw {@link SaslAuthenticationException} to provide custom error messages
         * to clients. But care should be taken to avoid including any information in the exception message that
         * should not be leaked to unauthenticated clients. It may be safer to throw {@link SaslException} in
         * some cases so that a standard error message is returned to clients.
         * </p>
         */
        @Override
        public byte[] evaluateResponse(byte[] response) throws SaslException, SaslAuthenticationException {
            /*
             * Message format (from https://tools.ietf.org/html/rfc4616):
             *
             * message   = [authzid] UTF8NUL authcid UTF8NUL passwd
             * authcid   = 1*SAFE ; MUST accept up to 255 octets
             * authzid   = 1*SAFE ; MUST accept up to 255 octets
             * passwd    = 1*SAFE ; MUST accept up to 255 octets
             * UTF8NUL   = %x00 ; UTF-8 encoded NUL character
             *
             * SAFE      = UTF1 / UTF2 / UTF3 / UTF4
             *                ;; any UTF-8 encoded Unicode character except NUL
             */

            String[] tokens;
            try {
                tokens = new String(response, "UTF-8").split("\u0000");
            } catch (UnsupportedEncodingException e) {
                throw new SaslException("UTF-8 encoding not supported", e);
            }
            if (tokens.length != 3)
                throw new SaslException("Invalid SASL/PLAIN response: expected 3 tokens, got " + tokens.length);
            String authorizationIdFromClient = tokens[0];
            String username = tokens[1];
            String password = tokens[2];

            if (username.isEmpty()) {
                throw new SaslException("Authentication failed: username not specified");
            }
            if (password.isEmpty()) {
                throw new SaslException("Authentication failed: password not specified");
            }

            String expectedPassword = jaasContext.configEntryOption(JAAS_USER_PREFIX + username,
                    KafkaLoginModule.class.getName());
            if (!password.equals(expectedPassword)) {
                throw new SaslAuthenticationException("Authentication failed: Invalid username or password");
            }

            if (!authorizationIdFromClient.isEmpty() && !authorizationIdFromClient.equals(username))
                throw new SaslAuthenticationException("Authentication failed: Client requested an authorization id that is different from username");

            this.authorizationId = username;

            complete = true;
            return new byte[0];
        }

        @Override
        public String getAuthorizationID() {
            if (!complete)
                throw new IllegalStateException("Authentication exchange has not completed");
            return authorizationId;
        }

        @Override
        public String getMechanismName() {
            return PLAIN_MECHANISM;
        }

        @Override
        public Object getNegotiatedProperty(String propName) {
            if (!complete)
                throw new IllegalStateException("Authentication exchange has not completed");
            return null;
        }

        @Override
        public boolean isComplete() {
            return complete;
        }

        @Override
        public byte[] unwrap(byte[] incoming, int offset, int len) throws SaslException {
            if (!complete)
                throw new IllegalStateException("Authentication exchange has not completed");
            return Arrays.copyOfRange(incoming, offset, offset + len);
        }

        @Override
        public byte[] wrap(byte[] outgoing, int offset, int len) throws SaslException {
            if (!complete)
                throw new IllegalStateException("Authentication exchange has not completed");
            return Arrays.copyOfRange(outgoing, offset, offset + len);
        }

        @Override
        public void dispose() throws SaslException {
        }
    }

    public static class KafkaSaslServerFactory implements SaslServerFactory {

        @Override
        public SaslServer createSaslServer(String mechanism, String protocol, String serverName, Map<String, ?> props, CallbackHandler cbh)
                throws SaslException {

            if (!KafkaSaslServer.PLAIN_MECHANISM.equals(mechanism))
                throw new SaslException(String.format("Mechanism \'%s\' is not supported. Only PLAIN is supported.", mechanism));

            if (!(cbh instanceof SaslServerCallbackHandler))
                throw new SaslException("CallbackHandler must be of type SaslServerCallbackHandler, but it is: " + cbh.getClass());

            return new KafkaSaslServer(((SaslServerCallbackHandler) cbh).jaasContext());
        }

        @Override
        public String[] getMechanismNames(Map<String, ?> props) {
            if (props == null) return new String[]{KafkaSaslServer.PLAIN_MECHANISM};
            String noPlainText = (String) props.get(Sasl.POLICY_NOPLAINTEXT);
            if ("true".equals(noPlainText))
                return new String[]{};
            else
                return new String[]{KafkaSaslServer.PLAIN_MECHANISM};
        }
    }

    public static class KafkaSaslServerProvider extends Provider {
        private final AteDelegate d = AteDelegate.get();

        @SuppressWarnings("deprecation")
        protected KafkaSaslServerProvider() {
            super("Ate SASL/PLAIN Server Provider", 1.0, "Ate SASL/PLAIN Server Provider for Kafka");
            put("SaslServerFactory." + KafkaSaslServer.PLAIN_MECHANISM, KafkaSaslServerFactory.class.getName());
        }

        public static void initialize() {
            Security.addProvider(new KafkaSaslServerProvider());
        }
    }
}
