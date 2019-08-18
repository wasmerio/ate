package com.tokera.ate.test.dao;

import com.tokera.ate.common.UUIDTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.dto.msg.MessageDataHeaderDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.events.PartitionSeedingEvent;
import com.tokera.ate.io.repo.DataPartitionChain;
import com.tokera.ate.scopes.Startup;
import com.tokera.ate.units.Hash;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;

import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.event.Observes;
import java.util.UUID;

@Startup
@ApplicationScoped
public class SeedingDelegate {
    private AteDelegate d = AteDelegate.get();
    private @MonotonicNonNull PrivateKeyWithSeedDto rootkey;

    public PrivateKeyWithSeedDto getRootKey() {
        assert rootkey != null : "@AssumeAssertion(nullness): Must not be null";
        return rootkey;
    }

    @PostConstruct
    public void init() {
        rootkey = d.encryptor.genSignKeyAndSeed();
    }

    public void onPartitionSeeding(@Observes PartitionSeedingEvent event) {
        DataPartitionChain chain = event.getChain();

        // Add the root key into the chain of trust
        assert rootkey != null : "@AssumeAssertion(nullness): Must not be null";
        chain.addTrustKey(rootkey.key(), d.genericLogger);

        // Add a dummy record for the root account
        MessageDataHeaderDto header = new MessageDataHeaderDto(
                UUIDTools.generateUUID("mycompany.org"),
                UUID.randomUUID(),
                UUID.randomUUID(),
                null,
                MyAccount.class);

        // Allow root key to edit the root account
        @Hash String hash = rootkey.publicHash();
        assert hash != null : "@AssumeAssertion(nullness): Must not be null";
        header.getAllowWrite().add(hash);
        chain.addTrustDataHeader(header, d.genericLogger);
    }
}
