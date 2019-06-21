package com.tokera.ate.test.rest;

import com.tokera.ate.ApiServer;
import com.tokera.ate.BootstrapApp;
import com.tokera.ate.BootstrapConfig;
import com.tokera.ate.client.RawClient;
import com.tokera.ate.client.RawClientBuilder;
import com.tokera.ate.client.TestTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.enumerations.DefaultStorageSystem;
import com.tokera.ate.test.dao.MyAccount;
import com.tokera.ate.test.dto.NewAccountDto;
import org.junit.jupiter.api.*;

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
        MessagePrivateKeyDto key = d.encryptor.genSignKey(512);

        String keyPem = d.encryptor.serializePublicKey64(key);
        if (keyPem == null) throw new WebApplicationException("Failed to generate private key for domain");
        d.implicitSecurity.getEnquireTxtOverride().put("tokauth.mycompany.org", keyPem);

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
        this.session.appendToPrefixForFs(ret.id + "/");
        this.session.appendToPrefixForRest(ret.id + "/");
    }
}
