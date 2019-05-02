package com.tokera.ate.delegates;

import com.tokera.ate.scopes.Startup;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import java.net.InetAddress;
import java.net.UnknownHostException;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.event.Observes;
import javax.inject.Inject;
import javax.ws.rs.WebApplicationException;

import com.tokera.ate.events.RegisterPublicTopicEvent;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.DomainName;
import com.tokera.ate.units.PlainText;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.xbill.DNS.Cache;
import org.xbill.DNS.DClass;
import org.xbill.DNS.Lookup;
import org.xbill.DNS.Record;
import org.xbill.DNS.SimpleResolver;
import org.xbill.DNS.TXTRecord;
import org.xbill.DNS.TextParseException;
import org.xbill.DNS.Type;

/**
 * Uses properties of the Internet to derive authentication and authorization rules
 */
@Startup
@ApplicationScoped
public class ImplicitSecurityDelegate {

    private AteDelegate d = AteDelegate.getUnsafe();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;
    
    private static final Cache g_dnsCache = new Cache();

    private ConcurrentHashMap<String, String> enquireOverride = new ConcurrentHashMap<>();
    private static Set<String> g_publicTopics = new HashSet<>();

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

    public void onRegisterPublicTopic(@Observes RegisterPublicTopicEvent topic)
    {
        g_publicTopics.add(topic.getName());
    }

    public boolean checkTopicIsPublic(String accDomain) {
        return g_publicTopics.contains(accDomain);
    }
    
    public @Nullable MessagePublicKeyDto enquireDomainKey(@DomainName String domain, boolean shouldThrow)
    {
        return enquireDomainKey(d.bootstrapConfig.implicitSecurityAlias, domain, shouldThrow);
    }
    
    public @Nullable MessagePublicKeyDto enquireDomainKey(String prefix, @DomainName String domain, boolean shouldThrow)
    {
        return enquireDomainKey(prefix, domain, shouldThrow, domain);
    }
    
    public @Nullable MessagePublicKeyDto enquireDomainKey(String prefix, @DomainName String domain, boolean shouldThrow, @Alias String alias)
    {
        @Nullable @PlainText String keyString = enquireDomainString(prefix + "." + domain, shouldThrow);
        if (keyString == null) return null;
        
        return d.encryptor.createPublicKey(keyString, alias);
    }
    
    public @Nullable @PlainText String enquireDomainString(@DomainName String domain, boolean shouldThrow)
    {
        String override = MapTools.getOrNull(enquireOverride, domain);
        if (override != null) {
            return override;
        }

        for (String publicTopic : this.g_publicTopics) {
            if ((d.bootstrapConfig.implicitSecurityAlias + "." + publicTopic).equals(domain)) {
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