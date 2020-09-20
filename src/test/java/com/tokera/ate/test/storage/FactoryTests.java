package com.tokera.ate.test.storage;

import com.tokera.ate.ApiServer;
import com.tokera.ate.BootstrapApp;
import com.tokera.ate.BootstrapConfig;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.delegates.RequestContextDelegate;
import com.tokera.ate.dto.WeldInitializationConfig;
import com.tokera.ate.enumerations.DefaultStorageSystem;
import com.tokera.ate.io.task.TaskHandler;
import com.tokera.ate.test.dao.MyAccount;
import com.tokera.ate.test.dao.MyThing;
import org.jboss.weld.bootstrap.spi.BeanDiscoveryMode;
import org.jboss.weld.environment.se.Weld;
import org.jboss.weld.junit5.WeldInitiator;
import org.jboss.weld.junit5.WeldJunit5Extension;
import org.jboss.weld.junit5.WeldSetup;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;
import org.junit.jupiter.api.extension.ExtendWith;

import javax.enterprise.context.RequestScoped;

@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public class FactoryTests {

    @BeforeAll
    public void init() {
        BootstrapConfig config = ApiServer.startWeld(new WeldInitializationConfig<>(null, BootstrapApp.class));
        config.setLoggingMessageDrops(true);
        config.setDefaultStorageSystem(DefaultStorageSystem.LocalRam);
        config.setPingCheckOnStart(false);

        ApiServer.startApiServer(config);

        AteDelegate d = AteDelegate.get();

        // Build the default storage subsystem
        d.storageFactory.buildKafkaBackend()
                .addAccessLoggerLayer();
    }

    @Test
    public void testBackend() {
        AteDelegate d = AteDelegate.get();
        TaskHandler.enterRequestScopeAndInvoke(() -> {
            d.dataRepository.backend().touch();
        });
    }
}
