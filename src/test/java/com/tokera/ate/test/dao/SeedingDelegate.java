package com.tokera.ate.test.dao;

import com.tokera.ate.common.UUIDTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageDataHeaderDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.events.TopicSeedingEvent;
import com.tokera.ate.io.repo.DataTopic;
import com.tokera.ate.io.repo.DataTopicChain;
import com.tokera.ate.scopes.Startup;
import com.tokera.ate.units.Hash;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;

import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.event.Observes;
import java.util.HashSet;
import java.util.UUID;

@Startup
@ApplicationScoped
public class SeedingDelegate {
    private AteDelegate d = AteDelegate.get();
    private @MonotonicNonNull MessagePrivateKeyDto rootkey;

    public MessagePrivateKeyDto getRootKey() {
        assert rootkey != null : "@AssumeAssertion(nullness): Must not be null";
        return rootkey;
    }

    @PostConstruct
    public void init() {
        rootkey = d.encryptor.genSignKeyNtru(128);
    }

    public void onTopicSeeding(@Observes TopicSeedingEvent event) {
        DataTopicChain chain = event.getChain();

        // Add the root key into the chain of trust
        assert rootkey != null : "@AssumeAssertion(nullness): Must not be null";
        chain.addTrustKey(rootkey, d.genericLogger);

        // Add a dummy record for the root account
        MessageDataHeaderDto header = new MessageDataHeaderDto(
                UUIDTools.generateUUID(chain.getTopicName()),
                UUID.randomUUID(),
                null,
                MyAccount.class);

        // Allow root key to edit the root account
        @Hash String hash = rootkey.getPublicKeyHash();
        assert hash != null : "@AssumeAssertion(nullness): Must not be null";
        header.getAllowWrite().add(hash);
        chain.addTrustDataHeader(header, d.genericLogger);
    }
}
