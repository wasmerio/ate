package com.tokera.ate.delegates;

import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.providers.PartitionKeySerializer;
import com.tokera.ate.scopes.Startup;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import java.net.InetAddress;
import java.net.UnknownHostException;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.event.Observes;
import javax.inject.Inject;
import javax.ws.rs.WebApplicationException;

import com.tokera.ate.events.RegisterPublicTopicEvent;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.DomainName;
import com.tokera.ate.units.Hash;
import com.tokera.ate.units.PlainText;
import edu.emory.mathcs.backport.java.util.Arrays;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.xbill.DNS.*;

/**
 * Uses properties of the Internet to derive authentication and authorization rules
 */
@Startup
@ApplicationScoped
public class ImplicitSecurityDelegate {

    private AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;
    
    private static final Cache g_dnsCache = new Cache();

    private ConcurrentHashMap<String, String> enquireOverride = new ConcurrentHashMap<>();
    private static Set<String> g_publicPartitions = new HashSet<>();

    @SuppressWarnings("initialization.fields.uninitialized")
    private SimpleResolver m_resolver;
    
    static {
        g_dnsCache.setMaxNCache(300);
        g_dnsCache.setMaxCache(300);
        g_dnsCache.setMaxEntries(20000);
    }
    
    @PostConstruct
    public void init() {
        try {
            m_resolver = new SimpleResolver();
            m_resolver.setTCP(true);
            m_resolver.setTimeout(4);
            //m_resolver.setTSIGKey(null);
            m_resolver.setAddress(InetAddress.getByName("8.8.8.8"));
        } catch (UnknownHostException ex) {
            LOG.error(ex);
        }
    }

    public void onRegisterPublicPartition(@Observes RegisterPublicTopicEvent partition)
    {
        g_publicPartitions.add(partition.getName());
    }

    public boolean checkPartitionIsPublic(String accDomain) {
        return g_publicPartitions.contains(accDomain);
    }

    public @Nullable MessagePublicKeyDto enquireDomainKey(@DomainName String domain, boolean shouldThrow)
    {
        return enquireDomainKey(domain, shouldThrow, null);
    }

    public @Nullable MessagePublicKeyDto enquireDomainKey(@DomainName String domain, boolean shouldThrow, @Nullable IPartitionKey partitionKey)
    {
        return enquireDomainKey(d.bootstrapConfig.getImplicitAuthorityAlias(), domain, shouldThrow, partitionKey);
    }

    public @Nullable MessagePublicKeyDto enquireDomainKey(String prefix, @DomainName String domain, boolean shouldThrow)
    {
        return enquireDomainKey(prefix, domain, shouldThrow, domain, null);
    }

    public @Nullable MessagePublicKeyDto enquireDomainKey(String prefix, @DomainName String domain, boolean shouldThrow, @Nullable IPartitionKey partitionKey)
    {
        return enquireDomainKey(prefix, domain, shouldThrow, domain, partitionKey);
    }

    public @Nullable MessagePublicKeyDto enquireDomainKey(String prefix, @DomainName String domain, boolean shouldThrow, @Alias String alias)
    {
        return enquireDomainKey(prefix, domain, shouldThrow, alias, null);
    }

    public @Nullable MessagePublicKeyDto enquireDomainKey(String prefix, @DomainName String domain, boolean shouldThrow, @Alias String alias, @Nullable IPartitionKey partitionKey)
    {
        String domainStr = enquireDomainString(prefix + "." + domain, shouldThrow);
        if (domainStr == null) {
            if (shouldThrow) {
                throw new RuntimeException("No implicit authority found at domain name [" + prefix + "." + domain + "] (missing TXT record).");
            }
            return null;
        }

        MessagePublicKeyDto ret;
        if (partitionKey != null) {
            ret = d.headIO.publicKeyOrNull(partitionKey, domainStr);
        } else {
            ret = d.headIO.publicKeyOrNull(domainStr);
        }
        if (ret == null) {
            ret = d.currentRights.getRightsWrite()
                    .stream()
                    .filter(k -> domainStr.equals(k.getPublicKeyHash()))
                    .findFirst()
                    .orElse(null);
        }

        if (ret == null) {
            if (shouldThrow) {
                throw new RuntimeException("Unknown implicit authority found at domain name [" + prefix + "." + domain + "] (public key is missing with hash [" + domainStr + "]).");
            }
        } else {
            ret = new MessagePublicKeyDto(ret);
            if (alias != null) {
                ret.setAlias(alias);
            }
        }

        return ret;
    }

    public String generateDnsTxtRecord(MessagePublicKeyDto key) {
        return generateDnsTxtRecord(key, d.requestContext.getPartitionKeyScopeOrNull());
    }

    public String generateDnsTxtRecord(MessagePublicKeyDto key, @Nullable IPartitionKey partitionKey) {
        if (partitionKey == null) {
            return key.getPublicKeyHash();
        }
        if (d.headIO.publicKeyOrNull(partitionKey, key.getPublicKeyHash()) == null) {
            d.headIO.merge(partitionKey, key);
        }
        String partitionKeyTxt = new PartitionKeySerializer().write(partitionKey);
        return Base64.encodeBase64URLSafeString(partitionKeyTxt.getBytes()) + ":" + key.getPublicKeyHash();
    }

    public List<String> enquireDomainAddresses(@DomainName String domain, boolean shouldThrow) {
        if (domain.contains(":")) {
            String[] comps = domain.split(":");
            if (comps.length >= 1) domain = comps[0];
        }
        if (domain.endsWith(".") == false) domain += ".";

        if ("localhost.".equalsIgnoreCase(domain)) {
            return Collections.singletonList("127.0.0.1");
        }

        try {

            Lookup lookup = new Lookup(domain, Type.ANY, DClass.IN);
            lookup.setResolver(m_resolver);
            lookup.setCache(g_dnsCache);

            final Record[] records = lookup.run();
            if (lookup.getResult() != Lookup.SUCCESSFUL) {
                if (shouldThrow && lookup.getResult() == Lookup.UNRECOVERABLE) {
                    throw new WebApplicationException("Failed to lookup DNS record on [" + domain + "] - " + lookup.getErrorString());
                }
                this.LOG.debug("dns(" + domain + ")::" + lookup.getErrorString());
                return null;
            }

            List<String> ret = new ArrayList<>();
            for (Record record : records) {
                //this.LOG.info("dns(" + domain + ")::record(" + record.toString() + ")");

                if (record instanceof ARecord) {
                    ARecord a = (ARecord) record;
                    ret.add(a.getAddress().toString());
                }
                if (record instanceof AAAARecord) {
                    AAAARecord aaaa = (AAAARecord) record;
                    ret.add(aaaa.getAddress().toString());
                }
            }
            return ret;
        } catch (TextParseException ex) {
            if (shouldThrow) {
                throw new WebApplicationException(ex);
            }
            this.LOG.info("dns(" + domain + ")::" + ex.getMessage());
            return null;
        }
    }
    
    public @Nullable @PlainText String enquireDomainString(@DomainName String domain, boolean shouldThrow)
    {
        String override = MapTools.getOrNull(enquireOverride, domain);
        if (override != null) {
            return override;
        }

        for (String publicTopic : this.g_publicPartitions) {
            if ((d.bootstrapConfig.getImplicitAuthorityAlias() + "." + publicTopic).equals(domain)) {
                return null;
            }
        }

        try
        {
            @DomainName String implicitAuth = domain;
            if (implicitAuth.endsWith(".") == false) implicitAuth += ".";
            
            Lookup lookup = new Lookup(implicitAuth, Type.ANY, DClass.IN);
            lookup.setResolver(m_resolver);
            lookup.setCache(g_dnsCache);

            final Record[] records = lookup.run();
            if (lookup.getResult() != Lookup.SUCCESSFUL) {
                if (shouldThrow && lookup.getResult() == Lookup.UNRECOVERABLE) {
                    throw new WebApplicationException("Failed to lookup DNS record on [" + domain + "] - " + lookup.getErrorString());
                }
                this.LOG.debug("dns(" + domain + ")::" + lookup.getErrorString());
                return null;
            }
            
            for (Record record : records) {
                //this.LOG.info("dns(" + domain + ")::record(" + record.toString() + ")");
                
                if (record instanceof TXTRecord) {
                    TXTRecord txt = (TXTRecord)record;
                    
                    final List strings = txt.getStrings();
                    if (strings.isEmpty()) {
                        continue;
                    }

                    StringBuilder sb = new StringBuilder();
                    for (Object str : strings) {
                        if (str == null) continue;
                        sb.append(str.toString());
                    }
                    return sb.toString();
                }
            }

            //this.LOG.info("dns(" + domain + ")::no_record");
            return null;
        } catch (TextParseException ex) {
            if (shouldThrow) {
                throw new WebApplicationException(ex);
            }
            this.LOG.info("dns(" + domain + ")::" + ex.getMessage());
            return null;
        }
    }

    public ConcurrentHashMap<String, String> getEnquireOverride() {
        return enquireOverride;
    }
}