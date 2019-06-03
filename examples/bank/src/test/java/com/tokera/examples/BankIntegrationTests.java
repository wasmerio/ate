package com.tokera.examples;

import com.tokera.ate.ApiServer;
import com.tokera.ate.client.RawClient;
import com.tokera.ate.client.RawClientBuilder;
import com.tokera.ate.client.TestTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.examples.dao.Company;
import com.tokera.examples.dao.Individual;
import com.tokera.examples.dao.MonthlyActivity;
import com.tokera.examples.dto.CreateAssetRequest;
import com.tokera.examples.dto.RegistrationResponse;
import com.tokera.examples.dto.RootLoginRequest;
import com.tokera.examples.dto.TransactionToken;
import org.junit.jupiter.api.*;

import javax.validation.constraints.NotNull;
import javax.ws.rs.client.Entity;
import javax.ws.rs.core.MediaType;
import java.math.BigDecimal;
import java.util.UUID;

@TestInstance(TestInstance.Lifecycle.PER_CLASS)
@DisplayName("[Bank Integration Tests]")
@TestMethodOrder(MethodOrderer.OrderAnnotation.class)
public class BankIntegrationTests {

    @SuppressWarnings("initialization.fields.uninitialized")
    private @NotNull RawClient session;

    private RawClient rootSession;

    private MessagePrivateKeyDto coiningKey;
    private RawClient coiningSession;
    private RawClient individualSession;
    private UUID individualAccountId;

    private MessagePrivateKeyDto companyKey;
    private RawClient companySession;
    private String companyDomain = "example.tokera.com";
    private String coiningDomain = "coin.example.tokera.com";
    private UUID companyAccountId;


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
        RegistrationResponse response = createClient().restPost(
                "/register/individual",
                Entity.entity("joe.blog@gmail.com", MediaType.TEXT_PLAIN), RegistrationResponse.class);
        AteDelegate d = AteDelegate.get();
        d.genericLogger.info(d.yaml.serializeObj(response));

        this.individualAccountId = response.getAccountId();
        this.individualSession = new RawClientBuilder()
                .server("127.0.0.1")
                .port(8080)
                .prefixForRest("/rs/1-0")
                .withLoginPost("/login/token", Entity.entity(response.getToken(), MediaType.APPLICATION_XML))
                .build();
        d.genericLogger.info("individual-login: " + individualSession.getSession());
    }

    @Test
    @Order(2)
    @DisplayName("...reading transactions for individual")
    public void readIndividualTransactions() {
        MonthlyActivity response = this.individualSession.restGet("/account/" + this.individualAccountId + "/transactions", MonthlyActivity.class);
        AteDelegate d = AteDelegate.get();
        d.genericLogger.info(d.yaml.serializeObj(response));
    }

    @Test
    @Order(3)
    @DisplayName("...generating company key")
    public void generateCompanyKey() {
        AteDelegate d = AteDelegate.get();
        this.companyKey = d.encryptor.genSignKeyFromSeedWithAlias(256, "not_so_secret_secret", companyDomain);
        d.genericLogger.info("Put this DNS entry at auth." + companyDomain + ": " + this.companyKey.getPublicKeyHash());
    }

    @Test
    @Order(4)
    @DisplayName("...generating coining key")
    public void generateCoiningKey() {
        AteDelegate d = AteDelegate.get();
        this.coiningKey = d.encryptor.genSignKeyFromSeedWithAlias(256, "unobtainium", coiningDomain);
        d.genericLogger.info("Put this DNS entry at auth." + coiningDomain + ": " + this.coiningKey.getPublicKeyHash());
    }

    @Test
    @Order(5)
    @DisplayName("...root login with key")
    public void rootLogin() {
        RootLoginRequest request = new RootLoginRequest();
        request.getWriteRights().add(this.companyKey);
        request.setUsername("root@" + companyDomain);

        this.rootSession = new RawClientBuilder()
                .server("127.0.0.1")
                .port(8080)
                .prefixForRest("/rs/1-0")
                .withLoginPost("/login/root", Entity.entity(request, "text/yaml"))
                .build();

        AteDelegate d = AteDelegate.get();
        d.genericLogger.info("root-login: " + rootSession.getSession());
    }

    @Test
    @Order(6)
    @DisplayName("...creating an company account")
    public void createCompany() {
        RegistrationResponse response = this.rootSession.restPost(
                "/register/company",
                Entity.entity(this.companyDomain, MediaType.TEXT_PLAIN), RegistrationResponse.class);
        AteDelegate d = AteDelegate.get();

        d.genericLogger.info(d.yaml.serializeObj(response));

        this.companyAccountId = response.getAccountId();
        this.companySession = new RawClientBuilder()
                .server("127.0.0.1")
                .port(8080)
                .prefixForRest("/rs/1-0")
                .withLoginPost("/login/token", Entity.entity(response.getToken(), MediaType.APPLICATION_XML))
                .build();
        d.genericLogger.info("company-login: " + companySession.getSession());
    }

    @Test
    @Order(7)
    @DisplayName("...coining login with key")
    public void coiningLogin() {
        RootLoginRequest request = new RootLoginRequest();
        request.getWriteRights().add(this.coiningKey);
        request.setUsername("root@" + coiningDomain);

        this.coiningSession = new RawClientBuilder()
                .server("127.0.0.1")
                .port(8080)
                .prefixForRest("/rs/1-0")
                .withLoginPost("/login/root", Entity.entity(request, "text/yaml"))
                .build();

        AteDelegate d = AteDelegate.get();
        d.genericLogger.info("coining-login: " + coiningSession.getSession());
    }

    @Test
    @Order(8)
    @DisplayName("...printing money for individual")
    public void printMoney() {
        CreateAssetRequest request = new CreateAssetRequest(coiningDomain, BigDecimal.valueOf(1000));
        TransactionToken transactionToken = this.coiningSession
                .restPost("/money/print", Entity.entity(request, MediaType.APPLICATION_JSON), TransactionToken.class);

        MonthlyActivity monthly = this.individualSession
                .restPost("/account/" + individualAccountId + "/completeTransaction", Entity.entity(transactionToken, MediaType.APPLICATION_JSON), MonthlyActivity.class);

        AteDelegate d = AteDelegate.get();
        d.genericLogger.info(d.yaml.serializeObj(monthly));
    }
}
