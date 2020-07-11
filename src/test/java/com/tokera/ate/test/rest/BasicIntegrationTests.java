package com.tokera.ate.test.rest;

import com.tokera.ate.ApiServer;
import com.tokera.ate.BootstrapApp;
import com.tokera.ate.BootstrapConfig;
import com.tokera.ate.client.RawClient;
import com.tokera.ate.client.RawClientBuilder;
import com.tokera.ate.client.TestTools;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.dto.WeldInitializationConfig;
import com.tokera.ate.enumerations.DefaultStorageSystem;
import com.tokera.ate.test.dao.MyAccount;
import com.tokera.ate.test.dao.MyThing;
import com.tokera.ate.test.dao.SeedingDelegate;
import com.tokera.ate.test.dto.NewAccountDto;
import com.tokera.ate.test.dto.ThingsDto;
import org.eclipse.microprofile.faulttolerance.Bulkhead;
import org.junit.jupiter.api.*;

import javax.enterprise.inject.spi.CDI;
import javax.validation.constraints.NotNull;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.client.Entity;
import javax.ws.rs.core.MediaType;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.Future;

@TestMethodOrder(MethodOrderer.OrderAnnotation.class)
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public class BasicIntegrationTests {

    @SuppressWarnings("initialization.fields.uninitialized")
    private @NotNull RawClient session;

    private UUID accountId;
    private List<UUID> testSet = new ArrayList<>();

    public BasicIntegrationTests() {
        int testSize = 200;
        for (Integer n = 0; n < testSize; n++) {
            this.testSet.add(UUID.randomUUID());
        }
    }

    @BeforeAll
	public static void init() {
		ApiServer.setPreventZooKeeper(true);
		ApiServer.setPreventKafka(true);
		//AuditInterceptor.setPreventObscuring(true);

        BootstrapConfig config = ApiServer.startWeld(new WeldInitializationConfig<>(null, BootstrapApp.class).clearPackages());
        config.setLoggingMessageDrops(true);
        config.setDefaultStorageSystem(DefaultStorageSystem.LocalRam);
        //config.setDefaultStorageSystem(DefaultStorageSystem.Kafka);
        config.setPingCheckOnStart(false);
        config.setRestPortOverride(8082);

        ApiServer.startApiServer(config);

		AteDelegate d = AteDelegate.get();
		d.init();
		d.encryptor.touch();
	}

    @Test
    @Order(1)
    public void testUuidSerializer() {
        TestTools.restGet(null, "http://127.0.0.1:8082/rs/1-0/test/uuid").readEntity(UUID.class);
    }

    @Test
    @Order(2)
    public void testTimeout() {
        Assertions.assertThrows(WebApplicationException.class, () -> {
            TestTools.restGet(null, "http://127.0.0.1:8082/rs/1-0/test/timeout").readEntity(String.class);
        });

        TestTools.restGet(null, "http://127.0.0.1:8082/rs/1-0/test/no-timeout").readEntity(String.class);
    }

    @Test
    @Order(3)
    public void testCustomData() {
        String customData = TestTools.restGet(null, "http://127.0.0.1:8082/rs/1-0/test/custom-data").readEntity(String.class);

        Assertions.assertEquals(customData, "my-data");
    }

    @Test
    @Order(10)
    public void getAdminKey() {
        AteDelegate d = AteDelegate.get();
        PrivateKeyWithSeedDto key = CDI.current().select(SeedingDelegate.class).get().getRootKey();
        String keyVal = key.serialize();

        d.implicitSecurity.addEnquireTxtOverride("tokauth.mycompany.org", key.publicHash());

        this.session = new RawClientBuilder()
                .server("127.0.0.1")
                .port(8082)
                .prefixForRest("/rs/1-0")
                .withLoginPost("/acc/adminToken/john", Entity.entity(key, MediaType.APPLICATION_JSON_TYPE))
                .build();
    }

    @Test
    @Order(11)
    public void createAccount() {
        NewAccountDto newDetails = new NewAccountDto();
        newDetails.setEmail("test@mycompany.org");

        MyAccount ret = session.restPut("/acc/register", Entity.entity(newDetails, MediaType.APPLICATION_JSON_TYPE), MyAccount.class);
        this.accountId = ret.id;
        session.setPartitionKey(ret.partitionKey());
    }

    @Test
    @Order(11)
    public void touchAccount() {
        session.restGet("/acc/" + this.accountId + "/touch", MyAccount.class);
    }

    @Test
    @Order(12)
    public void parallismMerge() {
        LoggerHook.withNoWarningsOrErrors(() -> {
            List<Future<MyAccount>> waitFor = new ArrayList<>();
            testSet.stream().forEach(descVal -> {
                waitFor.add(session.restPostAsync("/acc/" + this.accountId + "/addThing", Entity.json(descVal), MyAccount.class));
            });

            for (Future<MyAccount> future : waitFor) {
                try {
                    future.get();
                } catch (InterruptedException | ExecutionException e) {
                    throw new RuntimeException(e);
                }
            }
        });
    }

    /*
    @Test
    @Order(13)
    public void getThings() {
        MyAccount acc = session.restGet("/acc/" + this.accountId, MyAccount.class);
        Assertions.assertEquals(testSet.size(), acc.strongThings.size());
        for (UUID descVal : testSet) {
            Assertions.assertTrue(acc.strongThings.contains(descVal));
        }

        ThingsDto things = session.restGet("/acc/" + this.accountId + "/things", ThingsDto.class);
        Assertions.assertEquals(testSet.size(), things.things.size());
    }
    */

    @Test
    @Order(14)
    public void forceMaintenance() {
        LoggerHook.withNoWarningsOrErrors(() -> {
            AteDelegate.get().dataMaintenance.forceMaintenanceNow();
        });

        MyAccount acc = session.restGet("/acc/" + this.accountId, MyAccount.class);
        Assertions.assertEquals(testSet.size(), acc.strongThings.size());
        for (UUID descVal : testSet) {
            Assertions.assertTrue(acc.strongThings.contains(descVal));
        }
    }

    @RepeatedTest(100)
    @Order(20)
    public void megaPing() {
        session.restGet("/test/ping", String.class);
    }

    @RepeatedTest(100)
    @Order(20)
    @Bulkhead(value=1, waitingTaskQueue = 1)
    public void getAccount() {
        session.restGet("/acc/" + this.accountId, MyAccount.class);
    }
}
