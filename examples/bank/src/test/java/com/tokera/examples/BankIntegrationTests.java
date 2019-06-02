package com.tokera.examples;

import com.tokera.ate.ApiServer;
import com.tokera.ate.client.RawClient;
import com.tokera.ate.client.RawClientBuilder;
import com.tokera.ate.client.TestTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.examples.dto.RootLoginRequest;
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

    private MessagePrivateKeyDto companyKey;
    private String companyDomain = "example.tokera.com";
    private RawClient rootSession;

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
    }

    private RawClient createClient() {
        return new RawClientBuilder()
                .server("127.0.0.1")
                .port(8080)
                .prefixForRest("/rs/1-0")
                .build();
    }

    @Test
    @Order(1)
    @DisplayName("...creating an individual account")
    public void createIndividual() {
        String ret = createClient().restPost(
                "/register/individual",
                Entity.entity("joe.blog@gmail.com", MediaType.TEXT_PLAIN), String.class);
        AteDelegate d = AteDelegate.get();
        d.genericLogger.info(ret);
    }

    @Test
    @Order(2)
    @DisplayName("...generating company key")
    public void generateCompanyKey() {
        AteDelegate d = AteDelegate.get();
        this.companyKey = d.encryptor.genSignKeyFromSeedWithAlias(256, "not_so_secret_secret", companyDomain);
        d.genericLogger.info("Put this DNS entry at auth." + companyDomain + ": " + this.companyKey.getPublicKeyHash());
    }

    @Test
    @Order(3)
    @DisplayName("...root login with key")
    public void rootLogin() {
        RootLoginRequest request = new RootLoginRequest();
        request.getWriteRights().add(this.companyKey);
        request.setUsername("root@example.tokera.com");

        this.rootSession = new RawClientBuilder()
                .server("127.0.0.1")
                .port(8080)
                .prefixForRest("/rs/1-0")
                .withLoginPost("/register/root-login", Entity.entity(request, "text/yaml"))
                .build();

        AteDelegate d = AteDelegate.get();
        d.genericLogger.info("root-login: " + rootSession.getSession());
    }

    @Test
    @Order(4)
    @DisplayName("...creating an company account")
    public void createCompany() {
        String ret = this.rootSession.restPost(
                "/register/company",
                Entity.entity(companyDomain, MediaType.TEXT_PLAIN), String.class);
        AteDelegate d = AteDelegate.get();
        d.genericLogger.info(ret);
    }
}
