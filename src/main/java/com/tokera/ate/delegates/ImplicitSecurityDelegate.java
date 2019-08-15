package com.tokera.ate.delegates;

import com.google.common.cache.CacheBuilder;
import com.google.common.cache.RemovalNotification;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.GenericPartitionKey;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.ate.enumerations.EnquireDomainKeyHandling;
import com.tokera.ate.exceptions.ImplicitAuthorityMissingException;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.repo.DataPartition;
import com.tokera.ate.providers.PartitionKeySerializer;
import com.tokera.ate.scopes.Startup;
import com.tokera.ate.units.Alias;
import com.tokera.ate.units.DomainName;
import com.tokera.ate.units.PlainText;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.xbill.DNS.*;

import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import javax.ws.rs.WebApplicationException;
import java.net.InetAddress;
import java.net.UnknownHostException;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.function.Function;

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

    private final ConcurrentHashMap<String, String> enquireTxtOverride = new ConcurrentHashMap<>();
    private final ConcurrentHashMap<String, List<String>> enquireAddressOverride = new ConcurrentHashMap<>();
    private final Map<String, MessagePublicKeyDto> embeddedKeys = new HashMap<>();
    private final com.google.common.cache.Cache<String, MessagePublicKeyDto> implicitAuthorityCache;

    @SuppressWarnings("initialization.fields.uninitialized")
    private SimpleResolver m_resolver;
    
    static {
        g_dnsCache.setMaxNCache(300);
        g_dnsCache.setMaxCache(300);
        g_dnsCache.setMaxEntries(20000);
    }

    public ImplicitSecurityDelegate() {
        this.implicitAuthorityCache = CacheBuilder.newBuilder()
                .expireAfterAccess(600, TimeUnit.SECONDS)
                .build();
    }

    @PostConstruct
    public void init() {
        try {
            m_resolver = new SimpleResolver();
            m_resolver.setTCP(true);
            m_resolver.setTimeout(4);
            //m_resolver.setTSIGKey(null);
            m_resolver.setAddress(InetAddress.getByName(d.bootstrapConfig.getDnsServer()));

            List<MessagePublicKeyDto> keys = this.loadEmbeddedKeys();
            for (MessagePublicKeyDto key : keys) {
                this.embeddedKeys.put(key.getPublicKeyHash(), key);
            }
        } catch (UnknownHostException ex) {
            LOG.error(ex);
        }
    }

    public @Nullable MessagePublicKeyDto enquireDomainKey(@DomainName String domain, EnquireDomainKeyHandling handling)
    {
        return enquireDomainKey(domain, handling, null);
    }

    public @Nullable MessagePublicKeyDto enquireDomainKey(@DomainName String domain, EnquireDomainKeyHandling handling, IPartitionKey partitionKey)
    {
        return enquireDomainKey(d.bootstrapConfig.getImplicitAuthorityAlias(), domain, handling, partitionKey);
    }

    public @Nullable MessagePublicKeyDto enquireDomainKey(@DomainName String domain, EnquireDomainKeyHandling handling, IPartitionKey partitionKey, Function<String, MessagePublicKeyDto> publicKeyResolver) {
        return enquireDomainKey(d.bootstrapConfig.getImplicitAuthorityAlias(), domain, handling, null, partitionKey, publicKeyResolver);
    }

    public @Nullable MessagePublicKeyDto enquireDomainKey(String prefix, @DomainName String domain, EnquireDomainKeyHandling handling)
    {
        return enquireDomainKey(prefix, domain, handling, domain, null, null);
    }

    public @Nullable MessagePublicKeyDto enquireDomainKey(String prefix, @DomainName String domain, EnquireDomainKeyHandling handling, @Nullable IPartitionKey partitionKey)
    {
        return enquireDomainKey(prefix, domain, handling, domain, partitionKey, null);
    }

    public @Nullable MessagePublicKeyDto enquireDomainKey(String prefix, @DomainName String domain, EnquireDomainKeyHandling handling, @Alias String alias)
    {
        return enquireDomainKey(prefix, domain, handling, alias, null, null);
    }

    public @Nullable MessagePublicKeyDto enquireDomainKey(String prefix, @DomainName String domain, EnquireDomainKeyHandling handling, @Nullable @Alias String alias, @Nullable IPartitionKey partitionKey, @Nullable Function<String, MessagePublicKeyDto> publicKeyResolver)
    {
        String fullDomain = prefix + "." + domain;
        try {
            return implicitAuthorityCache.get(fullDomain, () ->
            {
                String publicKeyHash = enquireDomainString(fullDomain, handling);
                if (publicKeyHash == null) {
                    if (handling == EnquireDomainKeyHandling.ThrowOnNull) {
                        throw new ImplicitAuthorityMissingException("No implicit authority found at domain name [" + fullDomain + "] (missing TXT record).");
                    } else {
                        return null;
                    }
                }

                MessagePublicKeyDto ret;
                if (publicKeyResolver != null) {
                    ret = publicKeyResolver.apply(publicKeyHash);
                } else {
                    if (partitionKey != null) {
                        ret = d.io.publicKeyOrNull(partitionKey, publicKeyHash);
                    } else {
                        ret = d.io.publicKeyOrNull(publicKeyHash);
                    }
                    if (ret == null) {
                        ret = d.currentRights.getRightsWrite()
                                .stream()
                                .filter(k -> publicKeyHash.equals(k.getPublicKeyHash()))
                                .findFirst()
                                .orElse(null);
                    }
                }

                if (ret == null) {
                    if (handling == EnquireDomainKeyHandling.ThrowOnNull) {
                        throw new ImplicitAuthorityMissingException("Unknown implicit authority found at domain name [" + fullDomain + "] (public key is missing with hash [" + publicKeyHash + "]).");
                    } else {
                        return null;
                    }
                }

                ret = new MessagePublicKeyDto(ret);
                if (alias != null) {
                    ret.setAlias(alias);
                }
                return ret;
            });
        } catch (ExecutionException e) {
            throw new WebApplicationException(e);
        } catch (ImplicitAuthorityMissingException e) {
            if (handling.shouldThrowOnError() || handling == EnquireDomainKeyHandling.ThrowOnNull) throw e;
            return null;
        }
    }

    public String generateDnsTxtRecord(MessagePublicKeyDto key) {
        return generateDnsTxtRecord(key, d.requestContext.getPartitionKeyScopeOrNull());
    }

    public String generateDnsTxtRecord(MessagePublicKeyDto key, IPartitionKey partitionKey) {
        if (partitionKey == null) {
            String ret = key.getPublicKeyHash();
            if (ret == null) throw new RuntimeException("Failed to generate the DNS TXT record entry as the hash of the public key could not be generated.");
        }
        if (d.io.publicKeyOrNull(partitionKey, key.getPublicKeyHash()) == null) {
            d.io.write(partitionKey, key);
        }

        String partitionKeyTxt = new PartitionKeySerializer().write(partitionKey);
        assert partitionKeyTxt != null : "@AssumeAssertion(nullness): Must not be null";
        return Base64.encodeBase64URLSafeString(partitionKeyTxt.getBytes()) + ":" + key.getPublicKeyHash();
    }

    public List<String> enquireDomainAddresses(@DomainName String domain, EnquireDomainKeyHandling handling) {
        if (domain.contains(":")) {
            String[] comps = domain.split(":");
            if (comps.length >= 1) domain = comps[0];
        }
        if (domain.endsWith(".") == false) domain += ".";

        List<String> override = MapTools.getOrNull(enquireAddressOverride, domain);
        if (override != null) {
            return override;
        }

        if ("127.0.0.1.".equals(domain)) {
            return Collections.singletonList("127.0.0.1");
        }
        if ("localhost.".equals(domain)) {
            return Collections.singletonList("localhost");
        }

        try {

            Lookup lookup = new Lookup(domain, Type.ANY, DClass.IN);
            lookup.setResolver(m_resolver);
            lookup.setCache(g_dnsCache);

            final Record[] records = lookup.run();
            if (lookup.getResult() != Lookup.SUCCESSFUL) {
                if (handling.shouldThrowOnError() && lookup.getResult() == Lookup.UNRECOVERABLE) {
                    throw new WebApplicationException("Failed to lookup DNS record on [" + domain + "] - " + lookup.getErrorString());
                }
                this.LOG.debug("dns(" + domain + ")::" + lookup.getErrorString());
                if (handling == EnquireDomainKeyHandling.ThrowOnNull) {
                    throw new ImplicitAuthorityMissingException("No domain TXT record found at [" + domain + "].");
                } else {
                    return new ArrayList<>();
                }
            }

            List<String> ret = new ArrayList<>();
            for (Record record : records) {
                //this.LOG.info("dns(" + domain + ")::record(" + record.toString() + ")");

                if (record instanceof ARecord) {
                    ARecord a = (ARecord) record;
                    ret.add(a.getAddress().getHostAddress().trim().toLowerCase());
                }
                if (record instanceof AAAARecord) {
                    AAAARecord aaaa = (AAAARecord) record;
                    ret.add("[" + aaaa.getAddress().getHostAddress().trim().toLowerCase() + "]");
                }
            }
            if (ret.size() <= 0 && handling == EnquireDomainKeyHandling.ThrowOnNull) {
                throw new ImplicitAuthorityMissingException("No domain TXT record found at [" + domain + "].");
            }
            Collections.sort(ret);
            return ret;
        } catch (TextParseException ex) {
            if (handling.shouldThrowOnError()) {
                throw new WebApplicationException(ex);
            }
            this.LOG.info("dns(" + domain + ")::" + ex.getMessage());
            if (handling == EnquireDomainKeyHandling.ThrowOnNull) {
                throw new ImplicitAuthorityMissingException("No domain TXT record found at [" + domain + "].");
            } else {
                return new ArrayList<>();
            }
        }
    }
    
    public @Nullable @PlainText String enquireDomainString(@DomainName String domain, EnquireDomainKeyHandling handling)
    {
        String override = MapTools.getOrNull(enquireTxtOverride, domain);
        if (override != null) {
            return override;
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
                if (handling.shouldThrowOnError() && lookup.getResult() == Lookup.UNRECOVERABLE) {
                    throw new WebApplicationException("Failed to lookup DNS record on [" + domain + "] - " + lookup.getErrorString());
                }
                this.LOG.debug("dns(" + domain + ")::" + lookup.getErrorString());
                if (handling == EnquireDomainKeyHandling.ThrowOnNull) {
                    throw new ImplicitAuthorityMissingException("No domain TXT record found at [" + domain + "].");
                } else {
                    return null;
                }
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
                    if (sb.length() <= 0) continue;
                    return sb.toString();
                }
            }

            //this.LOG.info("dns(" + domain + ")::no_record");
            if (handling == EnquireDomainKeyHandling.ThrowOnNull) {
                throw new ImplicitAuthorityMissingException("No domain TXT record found at [" + domain + "].");
            } else {
                return null;
            }
        } catch (TextParseException ex) {
            if (handling.shouldThrowOnError()) {
                throw new WebApplicationException(ex);
            }
            this.LOG.info("dns(" + domain + ")::" + ex.getMessage());
            if (handling == EnquireDomainKeyHandling.ThrowOnNull) {
                throw new ImplicitAuthorityMissingException("No domain TXT record found at [" + domain + "].");
            } else {
                return null;
            }
        }
    }

    public ConcurrentHashMap<String, String> getEnquireTxtOverride() {
        return enquireTxtOverride;
    }

    public ConcurrentHashMap<String, List<String>> getEnquireAddressOverride() {
        return enquireAddressOverride;
    }

    public @Nullable MessagePublicKeyDto findEmbeddedKeyOrNull(String hash)
    {
        return MapTools.getOrNull(this.embeddedKeys, hash);
    }

    public Collection<MessagePublicKeyDto> embeddedKeys() {
        return this.embeddedKeys.values();
    }

    private List<MessagePublicKeyDto> loadEmbeddedKeys() {
        return d.resourceFile.loadAll("embedded-keys/", MessagePublicKeyDto.class);
    }
}