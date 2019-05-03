package com.tokera.ate;

import com.esotericsoftware.yamlbeans.YamlReader;
import com.google.common.base.Stopwatch;
import com.tokera.ate.common.ApplicationConfigLoader;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.extensions.*;
import com.tokera.ate.io.repo.KafkaBridgeBuilder;
import com.tokera.ate.providers.ProcessBodyReader;
import com.tokera.ate.providers.ProcessBodyWriter;
import com.tokera.ate.providers.YamlProvider;
import io.undertow.Undertow;
import io.undertow.UndertowOptions;
import io.undertow.servlet.Servlets;
import io.undertow.servlet.api.DeploymentInfo;
import org.hibernate.validator.HibernateValidator;
import org.jboss.resteasy.cdi.CdiInjectorFactory;
import org.jboss.resteasy.cdi.ResteasyCdiExtension;
import org.jboss.resteasy.plugins.providers.*;
import org.jboss.resteasy.plugins.providers.html.HtmlRenderableWriter;
import org.jboss.resteasy.plugins.providers.jackson.ResteasyJackson2Provider;
import org.jboss.resteasy.plugins.providers.jaxb.*;
import org.jboss.resteasy.plugins.providers.multipart.MimeMultipartProvider;
import org.jboss.resteasy.plugins.providers.sse.SseEventOutputProvider;
import org.jboss.resteasy.plugins.server.undertow.UndertowJaxrsServer;
import org.jboss.resteasy.spi.ResteasyDeployment;
import org.jboss.weld.bootstrap.spi.BeanDiscoveryMode;
import org.jboss.weld.environment.se.Weld;
import org.jboss.weld.environment.se.WeldContainer;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import javax.enterprise.inject.spi.CDI;
import javax.validation.*;
import javax.ws.rs.ApplicationPath;
import javax.ws.rs.WebApplicationException;
import java.io.IOException;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.Arrays;
import java.util.List;
import java.util.Properties;
import java.util.concurrent.TimeUnit;
import java.util.stream.Collectors;

public class ApiServer {

    private final BootstrapConfig config;

    private static final Logger LOG = LoggerFactory.getLogger(ApiServer.class);

    private Validator validator;
    private UndertowJaxrsServer server = new UndertowJaxrsServer();
    private Integer port = 8080;

    private static boolean preventKafka = false;
    private static boolean preventZooKeeper = false;

    private ApiServer(BootstrapConfig config) {
        this.config = config;
        ValidatorFactory factory = Validation.buildDefaultValidatorFactory();
        validator = factory.getValidator();
    }

    public void stop() {
        LOG.info("Stopping Undertow server...");
        this.getServer().stop();
        LOG.info("Undertow server has stopped");
    }

    private static List<Class> getRestProviders(ResteasyCdiExtension cdiExtension) {
        List<Class> providers = cdiExtension.getProviders().stream().collect(Collectors.toList());

        // Add the other providers (if they are not added already)
        List<Class> check = Arrays.asList(
                HtmlRenderableWriter.class,
                StringTextStar.class,
                ResteasyJackson2Provider.class,
                YamlProvider.class,
                ProcessBodyReader.class,
                ProcessBodyWriter.class,
                DefaultNumberWriter.class,
                DefaultBooleanWriter.class,
                DefaultTextPlain.class,
                ByteArrayProvider.class,
                FileProvider.class,
                IIOImageProvider.class,
                InputStreamProvider.class,
                JaxrsFormProvider.class,
                ReaderProvider.class,
                ServerFormUrlEncodedProvider.class,
                SourceProvider.class,
                StreamingOutputProvider.class,
                HtmlRenderableWriter.class,
                CollectionProvider.class,
                JAXBElementProvider.class,
                JAXBXmlRootElementProvider.class,
                JAXBXmlSeeAlsoProvider.class,
                JAXBXmlTypeProvider.class,
                MimeMultipartProvider.class,
                SseEventOutputProvider.class

        );
        for (Class clazz : check) {
            if (providers.contains(clazz) == false) {
                providers.add(clazz);
            }
        }
        return providers;
    }

    public static BootstrapConfig startWeld() {
        ValidatorFactory validatorFactory = Validation.byProvider( HibernateValidator.class )
                .configure()
                .buildValidatorFactory();
        Validator validator = validatorFactory.getValidator();

        Configuration<?> config = Validation.byDefaultProvider().configure();
        config.parameterNameProvider(config.getDefaultParameterNameProvider());
        BootstrapConfiguration bootstrap = config.getBootstrapConfiguration();

        // Load the CDI extension
        Weld weld = new Weld();
        weld.setBeanDiscoveryMode(BeanDiscoveryMode.ANNOTATED);
        weld.enableDiscovery();
        weld.addBeanClass(YamlProvider.class);
        weld.addBeanClass(ProcessBodyReader.class);
        weld.addBeanClass(ProcessBodyWriter.class);
        weld.addBeanClass(ResteasyJackson2Provider.class);
        weld.addBeanClass(ZooServer.class);
        weld.addBeanClass(KafkaServer.class);
        weld.addExtension(new ResteasyCdiExtension());
        weld.addExtension(new YamlTagDiscoveryExtension());
        weld.addExtension(new DaoParentDiscoveryExtension());
        weld.addExtension(new StartupBeanExtension());
        weld.addExtension(new ResourceScopedExtension());
        weld.addExtension(new TokenScopeExtension());
        weld.addExtension(new SerializableObjectsExtension());
        weld.addPackages(   true,
                ResteasyJackson2Provider.class,
                YamlReader.class,
                HtmlRenderableWriter.class,
                StringTextStar.class,
                AteDelegate.class);
        WeldContainer cdi = weld.initialize();

        return cdi.select(BootstrapConfig.class).get();
    }

    public static ApiServer startApiServer(BootstrapConfig apiConfig)
    {
        // Rebuild the mega delegate
        AteDelegate d = AteDelegate.get();

        loadEncryptorSettings(apiConfig);

        // Build the default storage subsystem
        d.storageFactory.buildKafkaBackend()
                        .addCacheLayer()
                        .addAccessLoggerLayer();
        
        // Load the properties file
        Properties props = ApplicationConfigLoader.getInstance().getPropertiesByName(apiConfig.getPropertiesFile());
        if (props == null) throw new WebApplicationException("Failed to load the properties file for the Tokera system.");
        
        // Start the API server
        ApiServer apiServer = new ApiServer(apiConfig);
        apiServer.start();

        // Start zookeeper and kafka
        if ("true".equals(props.getOrDefault("zookeeper.server", "false").toString()) &&
                preventZooKeeper == false)
        {
            boolean shouldForce = "true".equals(props.getOrDefault("zookeeper.force", "false").toString());
            CDI.current().select(ZooServer.class).get().start(shouldForce);
        }
        if ("true".equals(props.getOrDefault("kafka.server", "false").toString()) &&
                preventKafka == false)
        {
            CDI.current().select(KafkaBridgeBuilder.class).get().touch();
            CDI.current().select(KafkaServer.class).get().start();
        }
        
        try {
            // Get the application path
            String appPath="default";
            for (ApplicationPath path : apiConfig.getApplicationClass().getAnnotationsByType(ApplicationPath.class)) {
                appPath = path.value();
            }
            
            // Load the Resteasy deployment
            ResteasyDeployment re = new ResteasyDeployment();
            re.setApplicationClass(apiConfig.getApplicationClass().getName());

            // Add the dependency injection
            ResteasyCdiExtension cdiExtension = CDI.current().select(ResteasyCdiExtension.class).get();
            List<Class> resources = cdiExtension.getResources().stream().collect(Collectors.toList());
            List<Class> providers = getRestProviders(cdiExtension);

            //re.setActualResourceClasses(cdiExtension.getResources());
            re.setInjectorFactoryClass(CdiInjectorFactory.class.getName());
            re.getActualResourceClasses().addAll(resources);
            re.getActualProviderClasses().addAll(providers);
            LOG.debug("RestEasy InjectorFactory=" + re.getInjectorFactoryClass());

            ClassLoader clazzLoader = ApiServer.class.getClassLoader();
            if (clazzLoader == null) {
                throw new WebApplicationException("ClassLoader for ApiServer is could not be found.");
            }

            // REST API Deployment
            DeploymentInfo di = apiServer.getServer().undertowDeployment(re, appPath)
                    .setContextPath(apiConfig.getRestApiPath())
                    .setDeploymentName(apiConfig.getDeploymentName())
                    .setClassLoader(clazzLoader)
                    //.addListener(Servlets.listener(org.jboss.weld.environment.servlet.Listener.class))
                    //.addListener(Servlets.listener(org.jboss.weld.servlet.WeldListener.class));
                    //.addListener(Servlets.listener(org.jboss.weld.module.web.servlet.WeldListener.class));
                    .addListener(Servlets.listener(org.jboss.weld.environment.servlet.Listener.class));
            apiServer.getServer().deploy(di);

            /*
            // Web Socket Deployments
            WebSocketDeploymentInfo webSocketDeploymentInfo = new WebSocketDeploymentInfo()
                    .addEndpoint(TerminalWebSocket.class);
            DeploymentInfo websocketDeployment = deployment()
                    .setContextPath(TokeraProperty.TOKAPI_SERVER_WEB_SOCKET_API_PATH)
                    .addServletContextAttribute(WebSocketDeploymentInfo.ATTRIBUTE_NAME, webSocketDeploymentInfo)
                    .setDeploymentName(TokeraProperty.TOKAPI_SERVER_WEB_SOCKET_DEPLOYMENT_NAME)
                    .setClassLoader(clazzLoader);
            apiServer.getServer().deploy(websocketDeployment);
            */
                        
        } catch (Throwable ex) {
            LOG.error("Exception while starting API Server", ex);
            throw ex;
        }

        if (apiConfig.isPingCheckOnStart()) {
            // Loop attempting to invoke the service until it comes online, this will
            // ensure everything is precached for a faster fast call
            Stopwatch timer = Stopwatch.createStarted();
            for (int n = 0; ; n++) {
                try {
                    URL url = new URL("http://localhost:" + apiServer.port + apiConfig.getRestApiPath() + "/1-0/" + apiConfig.getPingCheckUrl());
                    HttpURLConnection con = (HttpURLConnection) url.openConnection();
                    con.setRequestMethod("GET");
                    con.setConnectTimeout(500);
                    con.setReadTimeout(500);

                    int ret = con.getResponseCode();
                    if (ret >= 200 && ret < 300) break;

                    Thread.sleep(500);
                } catch (InterruptedException | IOException ex) {
                    String msg = ex.getMessage();
                    if (msg == null) msg = ex.getClass().getSimpleName();
                    if (timer.elapsed(TimeUnit.SECONDS) > 30) {
                        LOG.warn(msg, ex);
                        n = 0;
                    }
                }
            }
        }

        return apiServer;
    }

    public UndertowJaxrsServer getServer() {
        return server;
    }

    public void start() {
        
        // Load the properties file
        Properties props = ApplicationConfigLoader.getInstance().getPropertiesByName(config.getPropertiesFile());
        if (props == null) throw new WebApplicationException("Properties for for the Tokera system could not be found.");
        port = Integer.parseInt(props.getOrDefault("port", "8080").toString());

        Undertow.Builder serverBuilder = Undertow.builder()
                .setServerOption(UndertowOptions.ENABLE_HTTP2, "true".equals(MapTools.getOrNull(props, "http2")))
                .addHttpListener(port, props.getOrDefault("listen", "0.0.0.0").toString())
                .setIoThreads(Integer.parseInt(props.getOrDefault("io.threads", "32").toString()))
                .setWorkerThreads(Integer.parseInt(props.getOrDefault("worker.threads", "1024").toString()))
                .setBufferSize(Integer.parseInt(props.getOrDefault("buffer.size", "16384").toString()));
        LOG.info("Starting Undertow server...");
        server = server.start(serverBuilder);
        LOG.info("Undertow server has started");
    }

    public static void setPreventZooKeeper(boolean val) {
        preventZooKeeper = val;
    }

    public static void setPreventKafka(boolean val) {
        preventKafka = val;
    }

    private static void loadEncryptorSettings(BootstrapConfig apiConfig) {
        int c_KeyPreGenThreads = 0;
        int c_KeyPreGenDelay = 0;
        int c_KeyPreGen64 = 0;
        int c_KeyPreGen128 = 0;
        int c_KeyPreGen256 = 0;
        int c_AesPreGen128 = 0;
        int c_AesPreGen256 = 0;

        String propsName = System.getProperty(apiConfig.getPropertiesFile());
        if (propsName != null) {
            Properties props = ApplicationConfigLoader.getInstance().getPropertiesByName(propsName);
            if (props != null) {
                c_KeyPreGenThreads = Integer.parseInt(props.getOrDefault("keygen.threads", "6").toString());
                c_KeyPreGenDelay = Integer.parseInt(props.getOrDefault("keygen.delay", "60").toString());
                c_KeyPreGen64= Integer.parseInt(props.getOrDefault("keygen.prealloc.64", "80").toString());
                c_KeyPreGen128 = Integer.parseInt(props.getOrDefault("keygen.prealloc.128", "60").toString());
                c_KeyPreGen256 = Integer.parseInt(props.getOrDefault("keygen.prealloc.256", "20").toString());
                c_AesPreGen128 = Integer.parseInt(props.getOrDefault("aesgen.prealloc.128", "800").toString());
                c_AesPreGen256 = Integer.parseInt(props.getOrDefault("aesgen.prealloc.256", "200").toString());
            }
        }

        AteDelegate d = AteDelegate.get();
        d.encryptor.setKeyPreGenThreads(c_KeyPreGenThreads);
        d.encryptor.setKeyPreGenDelay(c_KeyPreGenDelay);
        d.encryptor.setKeyPreGen64(c_KeyPreGen64);
        d.encryptor.setKeyPreGen128(c_KeyPreGen128);
        d.encryptor.setKeyPreGen256(c_KeyPreGen256);
        d.encryptor.setAesPreGen128(c_AesPreGen128);
        d.encryptor.setAesPreGen256(c_AesPreGen256);
    }
}
