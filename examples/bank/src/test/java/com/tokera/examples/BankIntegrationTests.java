package com.tokera.examples;

import com.tokera.ate.ApiServer;
import com.tokera.ate.client.RawClient;
import com.tokera.ate.client.RawClientBuilder;
import com.tokera.ate.client.TestTools;
import com.tokera.ate.delegates.AteDelegate;
import org.junit.jupiter.api.*;

import javax.validation.constraints.NotNull;
import javax.ws.rs.client.Entity;
import javax.ws.rs.core.MediaType;

@TestInstance(TestInstance.Lifecycle.PER_CLASS)
@DisplayName("[Bank Integration Tests]")
@TestMethodOrder(MethodOrderer.OrderAnnotation.class)
public class BankIntegrationTests {

    @SuppressWarnings("initialization.fields.uninitialized")
    private @NotNull RawClient session;

    @BeforeAll
    public static void init() {
        ApiServer.setPreventZooKeeper(true);
        ApiServer.setPreventKafka(true);
        //AuditInterceptor.setPreventObscuring(true);

        ShareBankApp.main(new String[0]);

        AteDelegate d = AteDelegate.get();
        d.init();
        d.encryptor.touch();

        // Build a storage system in memory for testing purposes
        d.storageFactory.buildRamBackend()
                .addCacheLayer()
                .addAccessLoggerLayer();

        TestTools.initSeedTestKeys();
    }

    @Test
    @Order(1)
    @DisplayName("...creating an individual account")
    public void createIndividual() {
        String email = "joe.blog@gmail.com";
        String ret = new RawClientBuilder()
                .server("127.0.0.1")
                .port(8080)
                .prefixForRest("/rs/1-0")
                .build()
                .restPost("/register/individual", Entity.entity(email, MediaType.TEXT_PLAIN), String.class);
        AteDelegate d = AteDelegate.get();
        d.genericLogger.info(ret);
    }
}
