package com.tokera.ate.test.rest;

import com.tokera.ate.ApiServer;
import com.tokera.ate.BootstrapApp;
import com.tokera.ate.BootstrapConfig;
import com.tokera.ate.client.RawClient;
import com.tokera.ate.client.RawClientBuilder;
import com.tokera.ate.client.TestTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.enumerations.DefaultStorageSystem;
import com.tokera.ate.test.dao.MyAccount;
import com.tokera.ate.test.dao.SeedingDelegate;
import com.tokera.ate.test.dto.NewAccountDto;
import org.junit.jupiter.api.*;

import javax.enterprise.inject.spi.CDI;
import javax.inject.Inject;
import javax.validation.constraints.NotNull;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.client.Entity;
import javax.ws.rs.core.MediaType;
import java.util.UUID;

@TestMethodOrder(MethodOrderer.OrderAnnotation.class)
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public class BasicIntegrationTests {

    @SuppressWarnings("initialization.fields.uninitialized")
    private @NotNull RawClient session;

    private UUID accountId;

    @BeforeAll
	public static void init() {
		ApiServer.setPreventZooKeeper(true);
		ApiServer.setPreventKafka(true);
		//AuditInterceptor.setPreventObscuring(true);

        BootstrapConfig config = ApiServer.startWeld(null, BootstrapApp.class);
        config.setDefaultStorageSystem(DefaultStorageSystem.LocalRam);
        config.setPingCheckOnStart(false);

        ApiServer.startApiServer(config);

		AteDelegate d = AteDelegate.get();
		d.init();
		d.encryptor.touch();
	}

    @Test
    @Order(1)
    public void testUuidSerializer() {
        TestTools.restGet(null, "http://127.0.0.1:8080/rs/1-0/test/uuid").readEntity(UUID.class);
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
                .port(8080)
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

    @RepeatedTest(100)
    @Order(20)
    public void getAccount() {
        session.restGet("/acc/" + this.accountId, MyAccount.class);
    }
}
