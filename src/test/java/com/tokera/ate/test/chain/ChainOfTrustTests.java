package com.tokera.ate.test.chain;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.extensions.DaoParentDiscoveryExtension;
import com.tokera.ate.delegates.CurrentRightsDelegate;
import com.tokera.ate.dto.EffectivePermissions;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.io.repo.DataContainer;
import com.tokera.ate.io.repo.DataSignatureBuilder;
import com.tokera.ate.io.repo.DataTopicChain;
import com.tokera.ate.security.Encryptor;
import java.io.IOException;
import java.util.Random;
import java.util.UUID;
import javax.annotation.PostConstruct;
import javax.enterprise.context.RequestScoped;
import javax.inject.Inject;

import com.tokera.ate.client.TestTools;
import com.tokera.ate.test.dao.MyAccount;
import com.tokera.ate.test.dao.MyThing;
import com.tokera.ate.units.Hash;
import org.jboss.weld.bootstrap.spi.BeanDiscoveryMode;
import org.jboss.weld.environment.se.Weld;
import org.jboss.weld.junit5.WeldInitiator;
import org.jboss.weld.junit5.WeldJunit5Extension;
import org.jboss.weld.junit5.WeldSetup;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;
import org.junit.jupiter.api.extension.ExtendWith;
import org.spongycastle.crypto.InvalidCipherTextException;

@TestInstance(TestInstance.Lifecycle.PER_CLASS)
@ExtendWith(WeldJunit5Extension.class)
public class ChainOfTrustTests
{
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private Encryptor encryptor;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private DataSignatureBuilder builder;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private CurrentRightsDelegate request;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private DaoParentDiscoveryExtension daoParents;

    private DataTopicChain createChain()
    {
        DataTopicChain ret = new DataTopicChain(
                "tokera.com",
                daoParents.getAllowedParentsSimple(),
                daoParents.getAllowedParentFreeSimple());
        encryptor.touch();
        return ret;
    }

    @SuppressWarnings("argument.type.incompatible")
    @WeldSetup
    public WeldInitiator weld = WeldInitiator
            .from(new Weld()
                    .setBeanDiscoveryMode(BeanDiscoveryMode.ANNOTATED)
                    .enableDiscovery()
                    .addBeanClass(MyAccount.class)
                    .addBeanClass(MyThing.class))
            .activate(RequestScoped.class)
            .build();
    
    @Test
    public void seeding()
    {
        DataTopicChain chain = createChain();
        MessagePublicKeyDto trustedKeyWrite = encryptor.getTrustOfPublicWrite();
        chain.addTrustKey(trustedKeyWrite, null);

        @Hash String hash = trustedKeyWrite.getPublicKeyHash();
        assert hash != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(hash);
        
        byte[] bytes1 = chain.getPublicKeyBytes(hash);
        byte[] bytes2 = trustedKeyWrite.getPublicKeyBytes();

        TestTools.assertEqualAndNotNull(bytes1, bytes2);
    }
    
    //@Test
    public void addMany() throws IOException, InvalidCipherTextException
    {
        byte[] bytes1 = new byte[2000];
        new Random().nextBytes(bytes1);
        
        DataTopicChain chain = createChain();
        MessagePrivateKeyDto trustedKeyWrite = encryptor.getTrustOfPublicWrite();
        chain.addTrustKey(trustedKeyWrite, null);
        
        UUID rootId = UUID.randomUUID();
        MessageDataHeaderDto header = new MessageDataHeaderDto(
                rootId,
                UUID.randomUUID(),
                null,
                MyAccount.class);
        UUID version = header.getVersionOrThrow();

        @Hash String hash = trustedKeyWrite.getPublicKeyHash();
        assert hash != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(hash);

        header.getAllowWrite().add(hash);
        chain.addTrustDataHeader(header, LOG);
        
        EffectivePermissions permissions = new EffectivePermissions();
        permissions.rolesWrite.add(hash);
        permissions.anchorRolesWrite.add(hash);
        request.getRightsWrite().add(Encryptor.generatePublicKeyWrite());

        MessageDataDigestDto digest = builder.signDataMessage(header, bytes1, permissions);
        Assertions.assertTrue(digest != null);

        long index = 0L;

        header = new MessageDataHeaderDto(UUID.randomUUID(), UUID.randomUUID(), version, MyThing.class);
        header.setParentId(rootId);
        header.setInheritWrite(true);

        digest = builder.signDataMessage(header, bytes1, permissions);
        Assertions.assertTrue(digest != null);

        MessageDataDto data = new MessageDataDto(header, digest, bytes1);
        boolean accepted = chain.rcv(data.createBaseFlatBuffer(), new MessageMetaDto(0, index++, 0), LOG);
        Assertions.assertTrue(accepted);
        
        // Should be no more 2 seconds for high performance
        for (int n = 0; n < 200; n++)
        {
            header = new MessageDataHeaderDto(UUID.randomUUID(), UUID.randomUUID(), version, MyThing.class);
            header.setParentId(rootId);
            header.setInheritWrite(true);

            digest = builder.signDataMessage(header, bytes1, permissions);
            Assertions.assertTrue(digest != null);
            
            for (int x = 0; x < 100; x++) {
                data = new MessageDataDto(header, digest, bytes1);
                accepted = chain.rcv(data.createBaseFlatBuffer(), new MessageMetaDto(0, index++, 0), LOG);
                Assertions.assertTrue(accepted);
            }
        }
        
        // As we did not properly sign this row after changing the ID it
        // should fail when we attempt to read it
        header.setId(UUID.randomUUID());

        data = new MessageDataDto(header, digest, bytes1);
        chain.rcv(data.createBaseFlatBuffer(), new MessageMetaDto(0, index++, 0), LOG);

        DataContainer rcvdata = chain.getData(data.getHeader().getIdOrThrow(), LOG);
        Assertions.assertTrue(rcvdata == null);

        // Now if we actually sign it then it will be accepted
        digest = builder.signDataMessage(header, bytes1, permissions);
        Assertions.assertTrue(digest != null);

        data = new MessageDataDto(header, digest, bytes1);
        chain.rcv(data.createBaseFlatBuffer(), new MessageMetaDto(0, index++, 0), LOG);
        
        // Attempt to read it (which will perform the validation)
        rcvdata = chain.getData(data.getHeader().getIdOrThrow(), LOG);
        Assertions.assertTrue(rcvdata != null);
    }
}
