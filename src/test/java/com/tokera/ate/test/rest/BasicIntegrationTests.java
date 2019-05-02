package com.tokera.ate.test.rest;

import com.tokera.ate.ApiServer;
import com.tokera.ate.BootstrapConfig;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.test.TestTools;
import com.tokera.ate.test.dto.NewAccountDto;
import org.jboss.resteasy.client.jaxrs.ResteasyClient;
import org.junit.jupiter.api.*;

import javax.ws.rs.ClientErrorException;
import javax.ws.rs.client.Entity;
import javax.ws.rs.core.MediaType;
import javax.ws.rs.core.Response;

@TestMethodOrder(MethodOrderer.OrderAnnotation.class)
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public class BasicIntegrationTests {

    @BeforeAll
	public static void init() {
		ApiServer.setPreventZooKeeper(true);
		ApiServer.setPreventKafka(true);
		//AuditInterceptor.setPreventObscuring(true);

        BootstrapConfig config = new BootstrapConfig();
        config.domain = "mycompany.org";

        ApiServer.startApiServer(config);

		AteDelegate d = AteDelegate.get();
		d.init();
		d.encryptor.touch();

        // Build a storage system in memory for testing purposes
        d.storageFactory.buildRamBackend()
                .addCacheLayer()
                .addAccessLoggerLayer();

		//TestTools.initSeedKeys();
	}

    @Test
    @Order(1)
    public void getAdminKey() {
        MessagePrivateKeyDto key = AteDelegate.get().encryptor.genSignKeyNtru(128);

        ResteasyClient client = TestTools.buildClient();
        Response response = null;
        try {
            response = client.target("http://127.0.0.1:8080/rs/1-0/acc/adminToken/john@mycompany.org")
                    .request(MediaType.WILDCARD_TYPE)
                    .post(Entity.entity(key, MediaType.APPLICATION_JSON_TYPE));
        } catch (ClientErrorException e) {
            Response resp = e.getResponse();
            //System.out.println(resp.readEntity(String.class));
            resp.close();
            throw e;
        }

        if (response.getStatus() < 200 || response.getStatus() >= 300) {
            String errMsg = response.readEntity(String.class);
            Assertions.fail(errMsg);
        }

        String auth = response.getHeaderString("Authorization");
    }

    @Test
    @Order(2)
    public void createAccount() {
        NewAccountDto newDetails = new NewAccountDto();
        newDetails.setEmail("test@mycompany.com");

        ResteasyClient client = TestTools.buildClient();
        Response response = null;
        try {
            response = client.target("http://127.0.0.1:8080/rs/1-0/acc/register")
                    .request(MediaType.APPLICATION_JSON_TYPE)
                    .put(Entity.entity(newDetails, MediaType.APPLICATION_JSON_TYPE));
        } catch (ClientErrorException e) {
            Response resp = e.getResponse();
            //System.out.println(resp.readEntity(String.class));
            resp.close();
            throw e;
        }

        if (response.getStatus() < 200 || response.getStatus() >= 300) {
            String errMsg = response.readEntity(String.class);
            Assertions.fail(errMsg);
        }

        String auth = response.getHeaderString("Authorization");
    }
}
