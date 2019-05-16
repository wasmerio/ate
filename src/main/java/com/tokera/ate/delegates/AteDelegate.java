package com.tokera.ate.delegates;

import com.tokera.ate.BootstrapConfig;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.XmlUtils;
import com.tokera.ate.events.*;
import com.tokera.ate.extensions.SerializableObjectsExtension;
import com.tokera.ate.filters.*;
import com.tokera.ate.io.layers.HeadIO;
import com.tokera.ate.io.layers.MemoryCacheIO;
import com.tokera.ate.io.core.DaoHelper;
import com.tokera.ate.io.core.RequestAccessLog;
import com.tokera.ate.io.core.StorageSystemFactory;
import com.tokera.ate.filters.ResourceScopeInterceptor;
import com.tokera.ate.io.merge.DataMerger;
import com.tokera.ate.io.kafka.KafkaBridgeBuilder;
import com.tokera.ate.security.Encryptor;
import com.tokera.ate.extensions.DaoParentDiscoveryExtension;
import com.tokera.ate.extensions.YamlTagDiscoveryExtension;
import com.tokera.ate.io.core.TransactionCoordinator;
import com.tokera.ate.io.repo.*;
import com.tokera.ate.io.kafka.KafkaConfigTools;
import com.tokera.ate.qualifiers.FrontendStorageSystem;
import com.tokera.ate.security.EncryptKeyCachePerRequest;
import com.tokera.ate.security.TokenSecurity;
import org.checkerframework.checker.initialization.qual.UnknownInitialization;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.NonNull;

import javax.enterprise.event.Event;
import javax.enterprise.inject.spi.BeanManager;
import javax.enterprise.inject.spi.CDI;
import javax.enterprise.util.AnnotationLiteral;
import javax.enterprise.util.TypeLiteral;
import javax.ws.rs.WebApplicationException;
import java.lang.annotation.Annotation;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.locks.ReentrantLock;

/**
 * Delegate that reduces the amount of boiler plate injecting plus reduces the
 * amount of redirection over delegates and initialization steps for requests
 */
public class AteDelegate {
    public final Event<TokenStateChangedEvent> eventTokenChanged;
    public final Event<NewAccessRightsEvent> eventNewAccessRights;
    public final Event<TokenScopeChangedEvent> eventTokenScopeChanged;
    public final Event<TokenDiscoveryEvent> eventTokenDiscovery;
    public final Event<RightsDiscoverEvent> eventRightsDiscover;
    public final Event<RegisterPublicTopicEvent> eventRegisterPublicTopic;
    public final Event<TopicSeedingEvent> eventTopicSeeding;

    public final RequestContextDelegate requestContext;
    public final ResourceStatsDelegate resourceStats;
    public final ResourceInfoDelegate resourceInfo;
    public final DaoHelper daoHelper;
    public final Encryptor encryptor;
    public final ResourceScopeInterceptor resourceScopeInterceptor;
    public final BeanManager beanManager;
    public final StorageSystemFactory storageFactory;
    public final KafkaConfigTools kafkaConfig;
    public final CurrentTokenDelegate currentToken;
    public final YamlDelegate yaml;
    public final IObjectSerializer os;
    public final DaoParentDiscoveryExtension daoParents;
    public final YamlTagDiscoveryExtension yamlDiscovery;
    public final SerializableObjectsExtension serializableObjectsExtension;
    public final EncryptKeyCachePerRequest encryptKeyCachePerRequest;
    public final TokenSecurity tokenSecurity;
    public final ImplicitSecurityDelegate implicitSecurity;
    public final CurrentRightsDelegate currentRights;
    public final MemoryCacheIO memoryCacheIO;
    public final AuthorizationDelegate authorization;
    public final HeadIO headIO;
    public final TransactionCoordinator transaction;
    public final DataMerger merger;
    public final DataSerializer dataSerializer;
    public final DataSignatureBuilder dataSignatureBuilder;
    public final DataRepoConfig dataRepoConfig;
    public final DataRepository dataRepository;
    public final KafkaBridgeBuilder kafkaBridgeBuilder;
    public final XmlUtils xml;
    public final RequestAccessLog requestAccessLog;
    public final LoggingDelegate logging;
    public final AccessLogInterceptor accessLogInterceptor;
    public final AuthorityInterceptor authorityInterceptor;
    public final CorsInterceptor corsInterceptor;
    public final DefaultBootstrapInit defaultBootstrapInit;
    public final FixResteasyBug fixResteasyBug;
    public final PartitionKeyInterceptor partitionKeyInterceptor;
    public final TransactionInterceptor transactionInterceptor;
    public final LoggerHook genericLogger;
    public final BootstrapConfig bootstrapConfig;

    private static final AtomicInteger g_rebuilding = new AtomicInteger();

    protected static <@NonNull T> T getBean(Class<@NonNull T> clazz) {
        return CDI.current().select(clazz).get();
    }

    protected static <@NonNull T> T getBean(Class<@NonNull T> clazz, Annotation a1) {
        return CDI.current().select(clazz, a1).get();
    }

    protected static <T> Event<T> getEventBean(Class<T> clazz) {
        return CDI.current().select(new TypeLiteral<Event<T>>(){}).get();
    }

    private static final ReentrantLock g_instanceLock = new ReentrantLock();
    protected static @MonotonicNonNull AteDelegate g_instance;
    protected static @MonotonicNonNull @UnknownInitialization AteDelegate g_instanceInitializing;

    public static AteDelegate get() {
        return AteDelegate.get(AteDelegate.class);
    }

    @SuppressWarnings({"unchecked"})
    public static <T extends AteDelegate> T get(Class<T> clazz) {
        if (g_instance != null && clazz.isInstance(g_instance)) {
            return (T)g_instance;
        }
        g_instanceLock.lock();
        try {
            if (g_instance != null && clazz.isInstance(g_instance)) {
                return (T)g_instance;
            }

            T ret;
            try {
                ret = clazz.newInstance();
            } catch (InstantiationException | IllegalAccessException e) {
                throw new WebApplicationException(e);
            }
            g_instance = ret;
            g_instanceInitializing = ret;
            return ret;
        } finally {
            g_instanceLock.unlock();
        }
    }

    public static AteDelegate getUnsafe() {
        return AteDelegate.getUnsafe(AteDelegate.class);
    }

    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible", "cast.unsafe", "unchecked"})
    public static <T extends AteDelegate> T getUnsafe(Class<T> clazz) {
        if (g_instanceInitializing != null && clazz.isInstance(g_instanceInitializing)) {
            return (T)g_instanceInitializing;
        }
        g_instanceLock.lock();
        try {
            if (g_instanceInitializing != null && clazz.isInstance(g_instanceInitializing)) {
                return (T)g_instanceInitializing;
            }
            return get(clazz);
        } finally {
            g_instanceLock.unlock();
        }
    }

    public void init() {
        Object replace;
        g_rebuilding.incrementAndGet();
        try {
            replace = getClass().getConstructor().newInstance();
        } catch (InstantiationException | IllegalAccessException | InvocationTargetException | NoSuchMethodException e) {
            throw new WebApplicationException(e);
        } finally {
            g_rebuilding.decrementAndGet();
        }
        for (Field field : getClass().getFields()) {
            field.setAccessible(true);
            try {
                field.set(this, field.get(replace));
            } catch (IllegalAccessException e) {
                continue;
            }
        }
    }

    public AteDelegate() {
        if (g_rebuilding.get() == 0) {
            AteDelegate.g_instanceInitializing = this;
        }
        this.beanManager = getBean(BeanManager.class);
        this.eventTokenScopeChanged = getEventBean(TokenScopeChangedEvent.class);
        this.eventNewAccessRights = getEventBean(NewAccessRightsEvent.class);
        this.eventTokenChanged = getEventBean(TokenStateChangedEvent.class);
        this.eventTokenDiscovery = getEventBean(TokenDiscoveryEvent.class);
        this.eventRightsDiscover = getEventBean(RightsDiscoverEvent.class);
        this.eventRegisterPublicTopic = getEventBean(RegisterPublicTopicEvent.class);
        this.eventTopicSeeding = getEventBean(TopicSeedingEvent.class);
        this.requestContext = getBean(RequestContextDelegate.class);
        this.resourceStats = getBean(ResourceStatsDelegate.class);
        this.resourceInfo = getBean(ResourceInfoDelegate.class);
        this.storageFactory = getBean(StorageSystemFactory.class);
        this.daoHelper = getBean(DaoHelper.class);
        this.encryptor = getBean(Encryptor.class);
        this.kafkaConfig = getBean(KafkaConfigTools.class);
        this.resourceScopeInterceptor = getBean(ResourceScopeInterceptor.class);
        this.encryptKeyCachePerRequest = getBean(EncryptKeyCachePerRequest.class);
        this.currentToken = getBean(CurrentTokenDelegate.class);
        this.yaml = getBean(YamlDelegate.class);
        this.os = getBean(IObjectSerializer.class);
        this.implicitSecurity = getBean(ImplicitSecurityDelegate.class);
        this.daoParents = getBean(DaoParentDiscoveryExtension.class);
        this.yamlDiscovery = getBean(YamlTagDiscoveryExtension.class);
        this.tokenSecurity = getBean(TokenSecurity.class);
        this.currentRights = getBean(CurrentRightsDelegate.class);
        this.memoryCacheIO = getBean(MemoryCacheIO.class);
        this.authorization = getBean(AuthorizationDelegate.class);
        this.headIO = getBean(HeadIO.class, new AnnotationLiteral<FrontendStorageSystem>() {});
        this.transaction = getBean(TransactionCoordinator.class);
        this.merger = getBean(DataMerger.class);
        this.dataSerializer = getBean(DataSerializer.class);
        this.dataSignatureBuilder = getBean(DataSignatureBuilder.class);
        this.dataRepoConfig = getBean(DataRepoConfig.class);
        this.dataRepository = getBean(DataRepository.class);
        this.kafkaBridgeBuilder = getBean(KafkaBridgeBuilder.class);
        this.xml = getBean(XmlUtils.class);
        this.requestAccessLog = getBean(RequestAccessLog.class);
        this.logging = getBean(LoggingDelegate.class);
        this.accessLogInterceptor = getBean(AccessLogInterceptor.class);
        this.authorityInterceptor = getBean(AuthorityInterceptor.class);
        this.corsInterceptor = getBean(CorsInterceptor.class);
        this.defaultBootstrapInit = getBean(DefaultBootstrapInit.class);
        this.fixResteasyBug = getBean(FixResteasyBug.class);
        this.partitionKeyInterceptor = getBean(PartitionKeyInterceptor.class);
        this.transactionInterceptor = getBean(TransactionInterceptor.class);
        this.genericLogger = getBean(LoggerHook.class);
        this.serializableObjectsExtension = getBean(SerializableObjectsExtension.class);
        this.bootstrapConfig = getBean(BootstrapConfig.class);
    }
}