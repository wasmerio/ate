package com.tokera.ate.security;

import com.google.common.base.Charsets;
import com.google.common.collect.Iterables;
import com.google.common.collect.Lists;
import com.tokera.ate.BootstrapConfig;
import com.tokera.ate.dao.enumerations.KeyType;
import com.tokera.ate.dto.msg.MessageKeyPartDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.scopes.Startup;
import com.tokera.ate.io.api.IAteIO;
import com.tokera.ate.qualifiers.BackendStorageSystem;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.security.core.*;
import com.tokera.ate.security.core.XmssKeySerializer;
import com.tokera.ate.security.core.ntru_predictable.EncryptionKeyPairGenerator;
import com.tokera.ate.units.*;
import com.tokera.ate.dao.msg.MessagePrivateKey;
import com.tokera.ate.dao.msg.MessagePublicKey;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;

import java.io.ByteArrayOutputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.io.UnsupportedEncodingException;
import java.math.BigInteger;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.security.InvalidAlgorithmParameterException;
import java.security.InvalidKeyException;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.security.NoSuchProviderException;
import java.security.SecureRandom;
import java.util.*;
import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.stream.Collectors;
import javax.annotation.PostConstruct;
import javax.crypto.BadPaddingException;
import javax.crypto.Cipher;
import javax.crypto.IllegalBlockSizeException;
import javax.crypto.NoSuchPaddingException;
import javax.crypto.SecretKey;
import javax.crypto.ShortBufferException;
import javax.crypto.spec.GCMParameterSpec;
import javax.crypto.spec.IvParameterSpec;
import javax.crypto.spec.SecretKeySpec;
import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.inject.spi.CDI;
import javax.enterprise.util.AnnotationLiteral;
import javax.inject.Inject;
import javax.xml.bind.DatatypeConverter;

import org.apache.commons.codec.binary.Base64;
import org.apache.commons.lang.time.StopWatch;
import org.apache.kafka.common.utils.Utils;
import org.bouncycastle.crypto.*;
import org.bouncycastle.pqc.crypto.ExchangePair;
import org.bouncycastle.pqc.crypto.newhope.*;
import org.bouncycastle.pqc.crypto.ntru.*;
import org.bouncycastle.pqc.crypto.qtesla.*;
import org.bouncycastle.pqc.crypto.rainbow.*;
import org.bouncycastle.pqc.crypto.xmss.*;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.bouncycastle.crypto.digests.SHA256Digest;
import org.bouncycastle.crypto.digests.SHA512Digest;

/**
 * System used for all kinds of encryption steps that the storage system and other components need
 */
@Startup
@ApplicationScoped
public class Encryptor implements Runnable
{
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private BootstrapConfig config;

    @SuppressWarnings("initialization.fields.uninitialized")
    private static Encryptor g_Instance;
    @SuppressWarnings("initialization.fields.uninitialized")
    private static MessageDigest g_sha256digest;
    @SuppressWarnings("initialization.fields.uninitialized")
    private static MessageDigest g_md5digest;
    @SuppressWarnings("initialization.fields.uninitialized")
    private MessageDigest sha256digest;
    @SuppressWarnings("initialization.fields.uninitialized")
    private MessageDigest md5digest;
    
    public static final int GCM_NONCE_LENGTH = 12; // in bytes
    public static final int AES_KEY_SIZE = 128; // in bits
    public static final int AES_KEY_SIZE_BYTES = AES_KEY_SIZE / 8; // in bytes
    public static final int GCM_TAG_LENGTH = 16; // in bytes
    
    private final SecureRandom srandom = new SecureRandom();
    private final ArrayList<Thread> threads = new ArrayList<>();

    private int c_KeyPreGenThreads = 6;
    private int c_KeyPreGenDelay = 60;
    private int c_KeyPreGen64 = 80;
    private int c_KeyPreGen128 = 80;
    private int c_KeyPreGen256 = 20;
    private int c_AesPreGen128 = 800;
    private int c_AesPreGen256 = 200;
    private int c_AesPreGen512 = 100;
    
    // Public role that everyone has
    private @MonotonicNonNull MessagePrivateKeyDto trustOfPublicRead;
    private @MonotonicNonNull MessagePrivateKeyDto trustOfPublicWrite;

    private final ConcurrentLinkedQueue<MessagePrivateKeyDto> genSign64Queue = new ConcurrentLinkedQueue<>();
    private final ConcurrentLinkedQueue<MessagePrivateKeyDto> genSign128Queue = new ConcurrentLinkedQueue<>();
    private final ConcurrentLinkedQueue<MessagePrivateKeyDto> genSign256Queue = new ConcurrentLinkedQueue<>();
    private final ConcurrentLinkedQueue<MessagePrivateKeyDto> genEncrypt128Queue = new ConcurrentLinkedQueue<>();
    private final ConcurrentLinkedQueue<MessagePrivateKeyDto> genEncrypt256Queue = new ConcurrentLinkedQueue<>();
    private final ConcurrentLinkedQueue<@Secret String> genAes128Queue = new ConcurrentLinkedQueue<>();
    private final ConcurrentLinkedQueue<@Secret String> genAes256Queue = new ConcurrentLinkedQueue<>();
    private final ConcurrentLinkedQueue<@Secret String> genAes512Queue = new ConcurrentLinkedQueue<>();
    private final ConcurrentLinkedQueue<@Secret String> genSaltQueue = new ConcurrentLinkedQueue<>();

    private Set<Integer> validEncryptSizes = new HashSet<>(Lists.newArrayList(32, 64, 128, 192, 256, 512));

    public class KeyPairBytes
    {
        public final byte[] privateKey;
        public final byte[] publicKey;

        public KeyPairBytes(byte[] privateKey, byte[] publicKey) {
            this.privateKey = privateKey;
            this.publicKey = publicKey;
        }
    }
    
    static {
        try {
            g_sha256digest = MessageDigest.getInstance("SHA-256");
            g_md5digest = MessageDigest.getInstance("MD5");
        } catch (Exception ex) {
            throw new RuntimeException(ex);
        }
    }

    /**
     * @return Set of valid encryption sizes measured in bits for asymmetric encryption
     */
    public Set<Integer> getValidEncryptSizes() {
        return validEncryptSizes;
    }

    /**
     * @param validEncryptSizes Set of valid encryption sizes measured in bits for asymmetric encryption
     */
    public void setValidEncryptSizes(Set<Integer> validEncryptSizes) {
        this.validEncryptSizes = validEncryptSizes;
    }

    @PostConstruct
    public void init() {
        g_Instance = this;
        
        try {
            sha256digest = MessageDigest.getInstance("SHA-256");
            md5digest = MessageDigest.getInstance("MD5");
        } catch (Exception ex) {
            throw new RuntimeException(ex);
        }

        java.security.Security.addProvider(
                new org.bouncycastle.jce.provider.BouncyCastleProvider()
        );

        for (int n = 0; n < c_KeyPreGenThreads; n++) {
            Thread thread = new Thread(this);
            thread.setPriority(Thread.MIN_PRIORITY);
            thread.setDaemon(true);
            thread.start();
            threads.add(thread);
        }
    }

    public void setBootstrapConfig(BootstrapConfig config) {
        this.config = config;
    }

    public void setKeyPreGenThreads(int val) {
        this.c_KeyPreGenThreads = val;
    }

    public void setKeyPreGenDelay(int val) {
        this.c_KeyPreGenDelay = val;
    }

    public void setKeyPreGen64(int val) {
        this.c_KeyPreGen64 = val;
    }

    public void setKeyPreGen128(int val) {
        this.c_KeyPreGen128 = val;
    }

    public void setKeyPreGen256(int val) {
        this.c_KeyPreGen256 = val;
    }

    public void setAesPreGen128(int val) {
        this.c_AesPreGen128 = val;
    }

    public void setAesPreGen256(int val) {
        this.c_AesPreGen256 = val;
    }

    public void setAesPreGen512(int val) {
        this.c_AesPreGen512 = val;
    }

    @Override
    public void run() {
        Long errorWaitTime = 500L;
        Long startupWaitTime = 2000L;

        // Wait a little bit before we start
        synchronized (this) {
            try {
                wait(startupWaitTime);
            } catch (InterruptedException e) {
                LOG.warn(e);
            }
        }
        
        StopWatch timer = new StopWatch();
        timer.start();
        while (true) {
            try {
                // Perform all the generation that is required
                long delta = (timer.getTime()/1000L) - c_KeyPreGenDelay;
                if (delta > 0) {
                    long cap = 2L + (delta / 8L);
                    runGenerateKeys(cap);
                }
                
                // Wait for the need to acquire more toPutKeys
                synchronized (this) {
                    wait(4000);
                }
                
                errorWaitTime = 500L;
            } catch (Throwable ex) {
                //LOG.error(ex.getMessage(), ex);
                try {
                    Thread.sleep(errorWaitTime);
                } catch (InterruptedException ex1) {
                    LOG.warn(ex1);
                    break;
                }
                errorWaitTime *= 2L;
                if (errorWaitTime > 4000L) {
                    errorWaitTime = 4000L;
                }
            }
        }
    }
    
    private static Cipher getAesCipher()
    {
        try {
            return Cipher.getInstance("AES");
        } catch (NoSuchAlgorithmException | NoSuchPaddingException ex) {
            throw new RuntimeException(ex);
        }
    }
    
    private static Cipher getAesCipherCbc()
    {
        try {
            return Cipher.getInstance("AES/CBC/PKCS5PADDING");
        } catch (NoSuchAlgorithmException | NoSuchPaddingException ex) {
            throw new RuntimeException(ex);
        }
    }
    
    private static Cipher getAesCipherGcm()
    {
        try {
            return Cipher.getInstance("AES/GCM/NoPadding", "SunJCE");
        } catch (NoSuchAlgorithmException | NoSuchPaddingException | NoSuchProviderException ex) {
            throw new RuntimeException(ex);
        }
    }
    
    private void runGenerateKeys(long cap) {

        int cntSign64 = genSign64Queue.size();
        int cntSign128 = genSign128Queue.size();
        int cntSign256 = genSign256Queue.size();
        int cntEncrypt128 = genEncrypt128Queue.size();
        int cntEncrypt256 = genEncrypt256Queue.size();
        int cntAes128 = genAes128Queue.size();
        int cntAes256 = genAes256Queue.size();
        int cntAes512 = genAes512Queue.size();
        int cntSalt = genSaltQueue.size();
        
        for (;;)
        {
            boolean didGen = false;
            if (cntSign64 < c_KeyPreGen64 && cntSign64 < cap) {
                genSign64Queue.add(this.genSignKeyNow(64, config.getDefaultSigningTypes()));
                cntSign64++;
                didGen = true;
            }
            if (cntSign128 < c_KeyPreGen128 && cntSign128 < cap) {
                genSign128Queue.add(this.genSignKeyNow(128, config.getDefaultSigningTypes()));
                cntSign128++;
                didGen = true;
            }
            if (cntSign256 < c_KeyPreGen256 && cntSign256 < cap) {
                genSign256Queue.add(this.genSignKeyNow(256, config.getDefaultSigningTypes()));
                cntSign256++;
                didGen = true;
            }
            if (cntEncrypt128 < c_KeyPreGen128 && cntEncrypt128 < cap) {
                genEncrypt128Queue.add(this.genEncryptKeyNow(128, config.getDefaultEncryptTypes()));
                cntEncrypt128++;
                didGen = true;
            }
            if (cntEncrypt256 < c_KeyPreGen256 && cntEncrypt256 < cap) {
                genEncrypt256Queue.add(this.genEncryptKeyNow(256, config.getDefaultEncryptTypes()));
                cntEncrypt256++;
                didGen = true;
            }
            if (cntSalt < c_AesPreGen128 && cntSalt < cap) {
                genSaltQueue.add(new BigInteger(320, srandom).toString(16).toUpperCase());
                cntSalt++;
                didGen = true;
            }
            if (cntAes128 < c_AesPreGen128 && cntAes128 < cap) {
                genAes128Queue.add(this.generateSecret64Now(128));
                cntAes128++;
                didGen = true;
            }
            if (cntAes256 < c_AesPreGen256 && cntAes256 < cap) {
                genAes256Queue.add(this.generateSecret64Now(256));
                cntAes256++;
                didGen = true;
            }
            if (cntAes512 < c_AesPreGen512 && cntAes512 < cap) {
                genAes512Queue.add(this.generateSecret64Now(512));
                cntAes512++;
                didGen = true;
            }
            
            if (didGen == false) {
                break;
            }
        }
    }
    
    public void touch() {
    }
    
    public void moreKeys() {
        synchronized (this) {
            this.notify();
        }
    }
    
    public static Encryptor getInstance() {
        return g_Instance;
    }
    
    public @Secret String encryptCbc(@Secret String key, @Nullable @Salt String initVector, @PlainText String value) {
        try {
            if (initVector == null)
                initVector = "";
            
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            @Salt byte[] initHash = Arrays.copyOfRange(digest.digest(initVector.getBytes(StandardCharsets.UTF_8)), 0, 16);
            @Secret byte[] keyHash = Arrays.copyOfRange(digest.digest(key.getBytes(StandardCharsets.UTF_8)), 0, 16);
            
            IvParameterSpec iv = new IvParameterSpec(initHash);
            SecretKeySpec skeySpec = new SecretKeySpec(keyHash, "AES");

            Cipher cipher = Encryptor.getAesCipherCbc();
            cipher.init(Cipher.ENCRYPT_MODE, skeySpec, iv);

            @Secret byte[] encrypted = cipher.doFinal(value.getBytes());
            return Base64.encodeBase64URLSafeString(encrypted);
        } catch (InvalidAlgorithmParameterException | InvalidKeyException | NoSuchAlgorithmException | BadPaddingException | IllegalBlockSizeException ex) {
            throw new RuntimeException(ex);
        }
    }
    
    public @PlainText String decryptCbc(@Secret String key, @Nullable @Salt String initVector, @Secret String encrypted) {
        try {
            if (initVector == null)
                initVector = "";
            
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            @Salt byte[] initHash = Arrays.copyOfRange(digest.digest(initVector.getBytes(StandardCharsets.UTF_8)), 0, 16);
            @Secret byte[] keyHash = Arrays.copyOfRange(digest.digest(key.getBytes(StandardCharsets.UTF_8)), 0, 16);
            
            IvParameterSpec iv = new IvParameterSpec(initHash);
            SecretKeySpec skeySpec = new SecretKeySpec(keyHash, "AES");

            Cipher cipher = Encryptor.getAesCipherCbc();
            cipher.init(Cipher.DECRYPT_MODE, skeySpec, iv);

            @PlainText byte[] original = cipher.doFinal(Base64.decodeBase64(encrypted));

            return new String(original);
        } catch (InvalidAlgorithmParameterException | InvalidKeyException | NoSuchAlgorithmException | BadPaddingException | IllegalBlockSizeException ex) {
            throw new RuntimeException(ex);
        }
    }
    
    public @Secret String encryptGcm(@Secret byte[] key, @Salt String initVector, @PlainText String value) {
        return cipherGcm(key, initVector, value, Cipher.ENCRYPT_MODE);
    }
    
    public @PlainText String decryptGcm(@Secret byte[] key, @Salt String initVector, @Secret String value) {
        return cipherGcm(key, initVector, value, Cipher.DECRYPT_MODE);
    }
    
    private @Secret String cipherGcm(@Secret byte[] key, @Nullable @Salt String _initVector, @PlainText String value, int mode)
    {
        try
        {
            @Salt String initVector = _initVector;
            SecretKey secretKey = new SecretKeySpec(key, 0, key.length, "AES");
            Cipher cipher = Encryptor.getAesCipherGcm();
            
            if (initVector != null) {
                MessageDigest digest = MessageDigest.getInstance("SHA-256");

                @Salt byte[] initBytes = digest.digest(initVector.getBytes());
                if (initBytes.length > GCM_NONCE_LENGTH) initBytes = Arrays.copyOf(initBytes, GCM_NONCE_LENGTH);
                
                GCMParameterSpec spec = new GCMParameterSpec(GCM_TAG_LENGTH * 8, initBytes);
                cipher.init(mode, secretKey, spec);
            } else {
                cipher.init(mode, secretKey);
            }
            
            @Secret byte[] ret = cipher.doFinal(value.getBytes());
            return Base64.encodeBase64URLSafeString(ret);
            
        } catch (InvalidAlgorithmParameterException | InvalidKeyException | NoSuchAlgorithmException | BadPaddingException | IllegalBlockSizeException ex) {
            throw new RuntimeException(ex);
        }
    }
    
    public @Secret byte[] encryptAes(@Secret byte[] key, @PlainText byte[] value) {
        return cipherAes(key, ByteBuffer.wrap(value), Cipher.ENCRYPT_MODE);
    }
    
    public @Secret byte[] encryptAes(@Secret byte[] key, @PlainText ByteBuffer value) {
        return cipherAes(key, value, Cipher.ENCRYPT_MODE);
    }
    
    public @PlainText byte[] decryptAes(@Secret byte[] key, @Secret byte[] value) {
        return cipherAes(key, ByteBuffer.wrap(value), Cipher.DECRYPT_MODE);
    }
    
    public @PlainText byte[] decryptAes(@Secret byte[] key, @Secret ByteBuffer value) {
        return cipherAes(key, value, Cipher.DECRYPT_MODE);
    }
    
    private @Secret byte[] cipherAes(@Secret byte[] key, @PlainText ByteBuffer value, int mode)
    {
        try
        {
            SecretKey secretKey = new SecretKeySpec(key, 0, key.length, "AES");
            Cipher cipher = Encryptor.getAesCipher();
            cipher.init(mode, secretKey);
            
            int neededSize = cipher.getOutputSize(value.remaining());
            byte[] ret = new byte[neededSize];
            
            int amt = cipher.doFinal(value, ByteBuffer.wrap(ret));
            if (amt <= 0) return ret;

            if (amt != ret.length) {
                byte[] newRet = new byte[amt];
                System.arraycopy(ret,0, newRet, 0, amt);
                ret = newRet;
            }
            
            return ret;
            
        } catch (InvalidKeyException | BadPaddingException | IllegalBlockSizeException | ShortBufferException ex) {
            throw new RuntimeException(ex);
        }
    }

    public MessagePrivateKeyDto genSignKey()
    {
        return genSignKey(config.getDefaultSigningStrength());
    }

    public MessagePrivateKeyDto genSignKey(int keysize)
    {
        return genSignKeyWithAlias(keysize, null);
    }

    public MessagePrivateKeyDto genSignKeyWithAlias(@Nullable @Alias String alias)
    {
        return genSignKeyWithAlias(config.getDefaultSigningStrength(), alias);
    }

    public MessagePrivateKeyDto genSignKeyWithAlias(int keysize, @Nullable @Alias String _alias)
    {
        @Alias String alias = _alias;
        if (keysize == 64) {
            MessagePrivateKeyDto ret = this.genSign64Queue.poll();
            if (ret != null) {
                if (alias != null) ret.setAlias(alias);
                return ret;
            }
        }
        if (keysize == 128) {
            MessagePrivateKeyDto ret = this.genSign128Queue.poll();
            this.moreKeys();
            if (ret != null) {
                if (alias != null) ret.setAlias(alias);
                return ret;
            }
        }
        if (keysize == 256) {
            MessagePrivateKeyDto ret = this.genSign256Queue.poll();
            if (ret != null) {
                if (alias != null) ret.setAlias(alias);
                return ret;
            }
        }

        return genSignKeyNowWithAlias(keysize, config.getDefaultSigningTypes(), alias);
    }

    public MessagePrivateKeyDto genSignKeyNow(int keySize) {
        return genSignKeyNowWithAlias(keySize, config.getDefaultSigningTypes(),null);
    }

    public MessagePrivateKeyDto genSignKeyNowWithAlias(int keySize, @Nullable @Alias String alias) {
        return genSignKeyNowWithAlias(keySize, config.getDefaultSigningTypes(),alias);
    }

    public MessagePrivateKeyDto genSignKeyNow(int keySize, Iterable<KeyType> keyTypes) {
        return genSignKeyNowWithAlias(keySize, keyTypes,null);
    }

    public MessagePrivateKeyDto genSignKeyNowWithAlias(int keySize, Iterable<KeyType> keyTypes, @Nullable @Alias String alias) {
        List<MessageKeyPartDto> publicParts = new LinkedList<>();
        List<MessageKeyPartDto> privateParts = new LinkedList<>();

        for (KeyType keyType : keyTypes) {
            KeyPairBytes pair;
            switch (keyType) {
                case qtesla:
                    pair = genSignKeyQTeslaNow(keySize);
                    break;
                case xmssmt:
                    pair = genSignKeyXmssMtNow(keySize);
                    break;
                case rainbow:
                    pair = genSignKeyRainbowNow(keySize);
                    break;
                default:
                    throw new RuntimeException("The key type [" + keyType + "] is not supported as an asymmetric encryption key.");
            }
            publicParts.add(new MessageKeyPartDto(keyType, keySize, pair.publicKey));
            privateParts.add(new MessageKeyPartDto(keyType, keySize, pair.privateKey));
        }

        MessagePrivateKeyDto ret = new MessagePrivateKeyDto(publicParts, privateParts);
        if (alias != null) ret.setAlias(alias);
        return ret;
    }

    public MessagePrivateKeyDto genSignKeyFromSeed(String seed) {
        return this.genSignKeyFromSeed(config.getDefaultSigningStrength(), seed);
    }

    public MessagePrivateKeyDto genSignKeyFromSeed(int keySize, String seed) {
        return genSignKeyFromSeedWithAlias(keySize, config.getDefaultSigningTypes(), seed, null);
    }

    public MessagePrivateKeyDto genSignKeyFromSeedWithAlias(int keySize, String seed, @Nullable @Alias String alias) {
        return genSignKeyFromSeedWithAlias(keySize, config.getDefaultSigningTypes(), seed, alias);
    }

    public MessagePrivateKeyDto genSignKeyFromSeed(int keySize, Iterable<KeyType> keyTypes, String seed) {
        return genSignKeyFromSeedWithAlias(keySize, keyTypes, seed, null);
    }

    public MessagePrivateKeyDto genSignKeyFromSeedWithAlias(int keySize, Iterable<KeyType> keyTypes, String seed, @Nullable @Alias String alias) {
        IRandomFactory randomFactory = new PredictablyRandomFactory(seed);
        List<MessageKeyPartDto> publicParts = new LinkedList<>();
        List<MessageKeyPartDto> privateParts = new LinkedList<>();

        for (KeyType keyType : keyTypes) {
            KeyPairBytes pair;
            switch (keyType) {
                case qtesla:
                    pair = genSignKeyQTeslaNow(keySize, randomFactory);
                    break;
                case xmssmt:
                    pair = genSignKeyXmssMtNow(keySize, randomFactory);
                    break;
                case rainbow:
                    pair = genSignKeyRainbowNow(keySize, randomFactory);
                    break;
                default:
                    throw new RuntimeException("The key type [" + keyType + "] is not supported as an asymmetric encryption key.");
            }
            publicParts.add(new MessageKeyPartDto(keyType, keySize, pair.publicKey));
            privateParts.add(new MessageKeyPartDto(keyType, keySize, pair.privateKey));
        }

        MessagePrivateKeyDto ret = new MessagePrivateKeyDto(publicParts, privateParts);
        if (alias != null) ret.setAlias(alias);
        return ret;
    }

    public MessagePrivateKeyDto genEncryptKey() {
        return this.genEncryptKey(config.getDefaultEncryptionStrength());
    }
    
    public MessagePrivateKeyDto genEncryptKey(int keysize)
    {
        if (keysize == 128) {
            MessagePrivateKeyDto ret = this.genEncrypt128Queue.poll();
            this.moreKeys();
            if (ret != null) {
                return ret;
            }
        }
        if (keysize == 256) {
            MessagePrivateKeyDto ret = this.genEncrypt256Queue.poll();
            if (ret != null) return ret;
        }
        
        return genEncryptKeyNow(keysize, config.getDefaultEncryptTypes());
    }

    public MessagePrivateKeyDto genEncryptKeyWithAlias(@Nullable @Alias String alias)
    {
        return genEncryptKeyWithAlias(config.getDefaultEncryptionStrength(), alias);
    }

    public MessagePrivateKeyDto genEncryptKeyWithAlias(int keysize, @Nullable @Alias String _alias)
    {
        MessagePrivateKeyDto key = genEncryptKey(keysize);

        @Alias String alias = _alias;
        if (alias == null) return key;
        key.setAlias(alias);

        return key;
    }

    public MessagePrivateKeyDto genEncryptKeyNow(int keySize) {
        return genEncryptKeyNowWithAlias(keySize, config.getDefaultEncryptTypes(), null);
    }

    public MessagePrivateKeyDto genEncryptKeyNowWithAlias(int keySize, @Nullable @Alias String alias) {
        return genEncryptKeyNowWithAlias(keySize, config.getDefaultEncryptTypes(), alias);
    }

    public MessagePrivateKeyDto genEncryptKeyNow(int keySize, Iterable<KeyType> keyTypes) {
        return genEncryptKeyNowWithAlias(keySize, keyTypes, null);
    }

    public MessagePrivateKeyDto genEncryptKeyNowWithAlias(int keySize, Iterable<KeyType> keyTypes, @Nullable @Alias String alias) {
        if (Iterables.size(keyTypes) <= 0) {
            throw new RuntimeException("Generated encryption key must have at least one key type.");
        }

        List<MessageKeyPartDto> publicParts = new LinkedList<>();
        List<MessageKeyPartDto> privateParts = new LinkedList<>();

        for (KeyType keyType : keyTypes) {
            KeyPairBytes pair;
            switch (keyType) {
                case ntru:
                    pair = genEncryptKeyNtruNow(keySize);
                    break;
                case newhope:
                    pair = genEncryptKeyNewHopeNow(keySize);
                    break;
                default:
                    throw new RuntimeException("The key type [" + keyType + "] is not supported as an asymmetric encryption key.");
            }
            publicParts.add(new MessageKeyPartDto(keyType, keySize, pair.publicKey));
            privateParts.add(new MessageKeyPartDto(keyType, keySize, pair.privateKey));
        }

        MessagePrivateKeyDto ret = new MessagePrivateKeyDto(publicParts, privateParts);
        if (alias != null) ret.setAlias(alias);
        return ret;
    }

    public MessagePrivateKeyDto genEncryptKeyFromSeed(int keySize, String seed) {
        return genEncryptKeyFromSeedWithAlias(keySize, config.getDefaultEncryptTypes(), seed, null);
    }

    public MessagePrivateKeyDto genEncryptKeyFromSeedWithAlias(int keySize, String seed, @Nullable @Alias String alias) {
        return genEncryptKeyFromSeedWithAlias(keySize, config.getDefaultEncryptTypes(), seed, alias);
    }

    public MessagePrivateKeyDto genEncryptKeyFromSeed(int keySize, Iterable<KeyType> keyTypes, String seed) {
        return genEncryptKeyFromSeedWithAlias(keySize, keyTypes, seed, null);
    }

    public MessagePrivateKeyDto genEncryptKeyFromSeedWithAlias(int keySize, Iterable<KeyType> keyTypes, String seed, @Nullable @Alias String alias) {
        IRandomFactory randomFactory = new PredictablyRandomFactory(seed);

        if (Iterables.size(keyTypes) <= 0) {
            throw new RuntimeException("Generated encryption key must have at least one key type.");
        }

        List<MessageKeyPartDto> publicParts = new LinkedList<>();
        List<MessageKeyPartDto> privateParts = new LinkedList<>();

        for (KeyType keyType : keyTypes) {
            KeyPairBytes pair;
            switch (keyType) {
                case ntru:
                    pair = genEncryptKeyNtruNow(keySize, randomFactory);
                    break;
                case newhope:
                    pair = genEncryptKeyNewHopeNow(keySize, randomFactory);
                    break;
                default:
                    throw new RuntimeException("The key type [" + keyType + "] is not supported as an asymmetric encryption key.");
            }
            publicParts.add(new MessageKeyPartDto(keyType, keySize, pair.publicKey));
            privateParts.add(new MessageKeyPartDto(keyType, keySize, pair.privateKey));
        }

        MessagePrivateKeyDto ret = new MessagePrivateKeyDto(publicParts, privateParts);
        if (alias != null) ret.setAlias(alias);
        return ret;
    }

    public KeyPairBytes genEncryptKeyNtruFromSeed(int keysize, @Secret String seed)
    {
        return genEncryptKeyNtruNow(keysize, new PredictablyRandomFactory(seed));
    }

    public KeyPairBytes genEncryptKeyNtruNow(int keysize)
    {
        return genEncryptKeyNtruNow(keysize, new SecureRandomFactory(srandom));
    }

    private NTRUEncryptionKeyGenerationParameters getNtruEncryptParamtersForKeySize(int keySize) {
        switch (keySize) {
            case 512:
            case 256:
                return NTRUEncryptionKeyGenerationParameters.APR2011_743;
            case 192:
                return NTRUEncryptionKeyGenerationParameters.APR2011_743_FAST;
            case 128:
                return NTRUEncryptionKeyGenerationParameters.APR2011_439;
            default:
                throw new RuntimeException("Unknown NTRU key size(" + keySize + ")");
        }
    }
    
    public KeyPairBytes genEncryptKeyNtruNow(int keySize, IRandomFactory randomFactory)
    {
        for (int n = 0; n < 8; n++) {
            EncryptionKeyPairGenerator keyGen = new EncryptionKeyPairGenerator();
            keyGen.init(getNtruEncryptParamtersForKeySize(keySize));

            AsymmetricCipherKeyPair pair = keyGen.generateKeyPair(randomFactory);
            if (testNtruKey(pair, keySize) == false) {
                continue;
            }
            return extractKey(pair);
        }
        throw new RuntimeException("Failed to generate encryption key");
    }

    public @Secret byte[] encryptNtruWithPublic(@Secret byte[] key, @PlainText byte[] data, int keySize)
    {
        try {
            NTRUEncryptionKeyGenerationParameters ntruEncryptParams = getNtruEncryptParamtersForKeySize(keySize);
            NTRUEncryptionPublicKeyParameters priv = new NTRUEncryptionPublicKeyParameters(key, ntruEncryptParams.getEncryptionParameters());

            NTRUEngine engine = new NTRUEngine();
            engine.init(true, priv);

            return engine.processBlock(data, 0, data.length);

        } catch (InvalidCipherTextException ex) {
            throw new RuntimeException(ex);
        }
    }

    public @PlainText byte[] decryptNtruWithPrivate(@Secret byte[] key, @Secret byte[] data, int keySize) throws IOException, InvalidCipherTextException
    {
        NTRUEncryptionKeyGenerationParameters ntruEncryptParams = getNtruEncryptParamtersForKeySize(keySize);
        NTRUEncryptionPrivateKeyParameters priv = new NTRUEncryptionPrivateKeyParameters(key, ntruEncryptParams.getEncryptionParameters());

        NTRUEngine engine = new NTRUEngine();
        engine.init(false, priv);

        return engine.processBlock(data, 0, data.length);
    }
    
    private boolean testNtruKey(AsymmetricCipherKeyPair pair, int keySize) {
        
        NTRUEncryptionPrivateKeyParameters privateKey = (NTRUEncryptionPrivateKeyParameters) pair.getPrivate();
        NTRUEncryptionPublicKeyParameters publicKey = (NTRUEncryptionPublicKeyParameters) pair.getPublic();

        for (int n = 0; n < 10; n++) {
            byte[] test = Base64.decodeBase64(this.generateSecret64());

            try {
                byte[] encBytes = this.encryptNtruWithPublic(publicKey.getEncoded(), test, keySize);
                byte[] plainBytes = this.decryptNtruWithPrivate(privateKey.getEncoded(), encBytes, keySize);
                if (!Arrays.equals(test, plainBytes)) {
                    continue;
                }
                return true;
            } catch (Throwable ex) {
                return false;
            }
        }
        return false;
    }

    public KeyPairBytes genEncryptKeyNewHopeFromSeed(int keysize, String seed)
    {
        return genEncryptKeyNewHopeNow(keysize, new PredictablyRandomFactory(seed));
    }

    public KeyPairBytes genEncryptKeyNewHopeNow(int keysize, IRandomFactory randomFactory)
    {
        KeyGenerationParameters params = new KeyGenerationParameters(randomFactory.getRandom(), keysize);

        NHKeyPairGenerator gen = new NHKeyPairGenerator();
        gen.init(params);
        return extractKey(gen.generateKeyPair());
    }

    public KeyPairBytes genEncryptKeyNewHopeNow(int keysize)
    {
        return genEncryptKeyNewHopeNow(keysize, new SecureRandomFactory(srandom));
    }

    public @Secret byte[] encryptNewHopeWithPublic(@Secret byte[] publicKey, @PlainText byte[] data, int keySize)
    {
        NHPublicKeyParameters params = new NHPublicKeyParameters(publicKey);
        ExchangePair exchangeSecret = new NHExchangePairGenerator(srandom).generateExchange(params);
        byte[] encKey = exchangeSecret.getSharedValue();

        NHPublicKeyParameters keyExchangePublic = (NHPublicKeyParameters) (exchangeSecret.getPublicKey());
        byte[] pubData = keyExchangePublic.getPubData();
        byte[] encData = encryptAes(encKey, data);

        ByteBuffer bb = ByteBuffer.allocate(4 + pubData.length + encData.length);
        bb.putInt(pubData.length);
        bb.put(pubData);
        bb.put(encData);
        return bb.array();
    }

    public @PlainText byte[] decryptNewHopeWithPrivate(@Secret byte[] privateKey, @Secret byte[] data, int keySize)
    {
        short[] secData = new short[privateKey.length/2];
        ByteBuffer privateBB = ByteBuffer.wrap(privateKey);
        for (int index = 0; index < secData.length; index++) {
            secData[index] = privateBB.getShort();
        }

        ByteBuffer bb = ByteBuffer.wrap(data);
        int pubDataLength = bb.getInt();
        byte[] pubData = new byte[pubDataLength];
        byte[] encData = new byte[data.length - (4 + pubDataLength)];
        bb.get(pubData);
        bb.get(encData);

        NHAgreement nhAgreement = new NHAgreement();
        nhAgreement.init(new NHPrivateKeyParameters(secData));
        byte[] encKey = nhAgreement.calculateAgreement(new NHPublicKeyParameters(pubData));

        return this.decryptAes(encKey, encData);
    }

    @SuppressWarnings("deprecation")
    public @Secret byte[] encrypt(MessagePublicKeyDto publicKey, @PlainText byte[] data)
    {
        if (validEncryptSizes.contains(data.length*8) == false) {
            StringBuilder sb = new StringBuilder();
            for (Integer size : validEncryptSizes.stream().sorted().collect(Collectors.toList())) {
                if (sb.length() <= 0) {
                    sb.append(size);
                } else {
                    sb.append(", ");
                    sb.append(size);
                }
            }
            throw new RuntimeException("Data to be encrypted is not a valid size (" + sb.toString() + " bits) - consider wrapping an AES symmetric key instead of directly encrypting the data.");
        }

        @Secret byte[] ret = data;

        List<MessageKeyPartDto> parts = publicKey.getPublicParts();
        if (parts == null || parts.size() <= 0) {
            throw new RuntimeException("Failed to encrypt the data has the public key is empty.");
        }

        for (MessageKeyPartDto part : parts) {

            switch (part.getType()) {
                case ntru:
                    ret = encryptNtruWithPublic(part.getKeyBytes(), ret, part.getSize());
                    break;
                case newhope:
                    ret = encryptNewHopeWithPublic(part.getKeyBytes(), ret, part.getSize());
                    break;
                default:
                    throw new RuntimeException("Unknown encryption crypto algorithm: " + part.getType());
            }
        }

        return ret;
    }

    @SuppressWarnings("deprecation")
    public @PlainText byte[] decrypt(MessagePrivateKeyDto privateKey, @Secret byte[] data) throws IOException, InvalidCipherTextException
    {
        @PlainText byte[] ret = data;

        List<MessageKeyPartDto> parts = privateKey.getPrivateParts();
        if (parts == null || parts.size() <= 0) {
            throw new RuntimeException("Failed to decrypt the data has the public key is empty.");
        }

        for (MessageKeyPartDto part : Lists.reverse(parts)) {
            switch (part.getType()) {
                case ntru:
                    ret = decryptNtruWithPrivate(part.getKeyBytes(), ret, part.getSize());
                    break;
                case newhope:
                    ret = decryptNewHopeWithPrivate(part.getKeyBytes(), ret, part.getSize());
                    break;
                default:
                    throw new RuntimeException("Unknown encryption crypto algorithm: " + part.getType());
            }
        }

        return ret;
    }

    public KeyPairBytes genSignKeyQTeslaFromSeed(int keysize, String seed) {
        return genSignKeyQTeslaNow(keysize, new PredictablyRandomFactory(seed));
    }

    public KeyPairBytes genSignKeyQTeslaNow(int keysize) {
        return genSignKeyQTeslaNow(keysize, new SecureRandomFactory(srandom));
    }

    private int getQTeslaSecurityCategory(int keySize) {
        switch (keySize) {
            case 512:
                return QTESLASecurityCategory.PROVABLY_SECURE_III;
            case 256:
            case 192:
                return QTESLASecurityCategory.HEURISTIC_III_SIZE;
            case 128:
            case 64:
                return QTESLASecurityCategory.HEURISTIC_I;
            default:
                throw new RuntimeException("Unknown GMSS key size(" + keySize + ")");
        }
    }

    public KeyPairBytes genSignKeyQTeslaNow(int keysize, IRandomFactory randomFactory)
    {
        QTESLAKeyGenerationParameters params = new QTESLAKeyGenerationParameters(getQTeslaSecurityCategory(keysize), randomFactory.getRandom());
        QTESLAKeyPairGenerator gen = new QTESLAKeyPairGenerator();
        gen.init(params);
        return extractKey(gen.generateKeyPair());
    }

    public @Signature byte[] signQTesla(@Secret byte[] privateKey, @Hash byte[] digest, int keySize)
    {
        QTESLAPrivateKeyParameters params = new QTESLAPrivateKeyParameters(getQTeslaSecurityCategory(keySize), privateKey);
        QTESLASigner signer = new QTESLASigner();
        signer.init(true, params);
        return signer.generateSignature(digest);
    }

    public boolean verifyQTesla(@PEM byte[] publicKey, @Hash byte[] digest, @Signature byte[] sig, int keySize)
    {
        QTESLAPublicKeyParameters params = new QTESLAPublicKeyParameters(getQTeslaSecurityCategory(keySize), publicKey);
        QTESLASigner signer = new QTESLASigner();
        signer.init(false, params);
        return signer.verifySignature(digest, sig);
    }

    public KeyPairBytes genSignKeyRainbowFromSeed(int keySize, String seed) {
        return genSignKeyRainbowNow(keySize, new PredictablyRandomFactory(seed));
    }

    private RainbowParameters getRainbowParams(int keySize) {
        switch (keySize) {
            case 512:
                return new RainbowParameters();
            case 256:
                return new RainbowParameters();
            case 192:
                return new RainbowParameters();
            case 128:
            case 64:
                return new RainbowParameters();
            default:
                throw new RuntimeException("Unknown RAINBOW key size(" + keySize + ")");
        }
    }

    public KeyPairBytes genSignKeyRainbowNow(int keySize, IRandomFactory randomFactory)
    {
        SecureRandom keyRandom = randomFactory.getRandom();
        RainbowKeyGenerationParameters params = new RainbowKeyGenerationParameters(keyRandom, getRainbowParams(keySize));
        RainbowKeyPairGenerator gen = new RainbowKeyPairGenerator();
        gen.init(params);
        return extractKey(gen.generateKeyPair());
    }

    public KeyPairBytes genSignKeyRainbowNow(int keySize)
    {
        return genSignKeyRainbowNow(keySize, new SecureRandomFactory(srandom));
    }

    public @Signature byte[] signRainbow(@Secret byte[] privateKey, @Hash byte[] digest)
    {
        RainbowPrivateKeyParameters params = RainbowKeySerializer.deserializePrivate(privateKey);
        RainbowSigner signer = new RainbowSigner();
        signer.init(true, params);
        return signer.generateSignature(digest);
    }

    public boolean verifyRainbow(@PEM byte[] publicKey, @Hash byte[] digest, @Signature byte[] sig)
    {
        RainbowPublicKeyParameters params = RainbowKeySerializer.deserializePublic(publicKey);
        RainbowSigner signer = new RainbowSigner();
        signer.init(false, params);
        return signer.verifySignature(digest, sig);
    }

    public KeyPairBytes genSignKeyXmssMtFromSeed(int keysize, String seed) {
        return genSignKeyXmssMtNow(keysize, new PredictablyRandomFactory(seed));
    }

    public KeyPairBytes genSignKeyXmssMtNow(int keysize)
    {
        return genSignKeyXmssMtNow(keysize, new SecureRandomFactory(srandom));
    }

    public KeyPairBytes genSignKeyXmssMtNow(int keysize, IRandomFactory randomFactory)
    {
        SecureRandom keyRandom = randomFactory.getRandom();
        XMSSMTParameters params = new XMSSMTParameters(20, 10, new SHA512Digest());

        XMSSMTKeyPairGenerator gen = new XMSSMTKeyPairGenerator();
        gen.init(new XMSSMTKeyGenerationParameters(params, keyRandom));
        return extractKey(gen.generateKeyPair());
    }

    public @Signature byte[] signXmssMt(@Secret byte[] privateKey, @Hash byte[] digest)
    {
        int hash = Utils.murmur2(digest);
        XMSSMTPrivateKeyParameters paramsPrivate = XmssKeySerializer.deserializePrivate(privateKey, hash);

        XMSSMTSigner signer = new XMSSMTSigner();
        signer.init(true, paramsPrivate);
        return signer.generateSignature(digest);
    }

    public boolean verifyXmssMt(@PEM byte[] publicKey, @Hash byte[] digest, @Signature byte[] sig)
    {
        XMSSMTPublicKeyParameters paramPublic = XmssKeySerializer.deserializePublic(publicKey);

        XMSSMTSigner signer = new XMSSMTSigner();
        signer.init(false, paramPublic);
        return signer.verifySignature(digest, sig);
    }

    public @Signature byte[] sign(MessagePrivateKeyDto privateKey, @Hash byte[] digest)
    {
        List<@Signature byte[]> ret = new ArrayList<>();
        for (MessageKeyPartDto part : privateKey.getPrivateParts()) {

            switch (part.getType()) {
                case qtesla:
                    ret.add(signQTesla(part.getKeyBytes(), digest, part.getSize()));
                    break;
                case xmssmt:
                    ret.add(signXmssMt(part.getKeyBytes(), digest));
                    break;
                case rainbow:
                    ret.add(signRainbow(part.getKeyBytes(), digest));
                    break;
                default:
                    throw new RuntimeException("Unknown signing crypto algorithm: " + part.getType());
            }
        }

        ByteArrayOutputStream baos = new ByteArrayOutputStream();
        DataOutputStream dos = new DataOutputStream(baos);
        try {
            for (@Signature byte[] sig : ret) {
                dos.writeInt(sig.length);
                dos.write(sig);
            }
        } catch (IOException e) {
            throw new RuntimeException(e);
        }

        return baos.toByteArray();
    }

    @SuppressWarnings("deprecation")
    public boolean verify(MessagePublicKeyDto publicKey, @Hash byte[] digest, @Signature byte[] sigs)
    {
        ByteBuffer bb = ByteBuffer.wrap(sigs);

        List<MessageKeyPartDto> parts = publicKey.getPublicParts();
        if (parts == null || parts.size() <= 0) return false;

        for (MessageKeyPartDto part : parts) {

            int len = bb.getInt();
            if (len <= 0 || len > bb.remaining()) {
                return false;
            }
            byte[] partSig = new byte[len];
            bb.get(partSig);

            switch (part.getType()) {
                case qtesla:
                    if (verifyQTesla(part.getKeyBytes(), digest, partSig, part.getSize()) == false) {
                        return false;
                    }
                    break;
                case xmssmt:
                    if (verifyXmssMt(part.getKeyBytes(), digest, partSig) == false) {
                        return false;
                    }
                    break;
                case rainbow:
                    if (verifyRainbow(part.getKeyBytes(), digest, partSig) == false) {
                        return false;
                    }
                    break;
                default:
                    throw new RuntimeException("Unknown signing crypto algorithm: " + part.getType());
            }
        }

        if (bb.remaining() > 0) {
            return false;
        }

        return true;
    }
    
    public @Hash byte[] hashSha(@PlainText String data) {
        return hashSha(null, data);
    }
    
    public @Hash byte[] hashSha(@Nullable @Salt String seed, @PlainText String data) {
        if (seed != null) {
            return hashSha(seed.getBytes(Charsets.US_ASCII), data.getBytes(Charsets.US_ASCII));
        } else {
            return hashSha(data.getBytes(Charsets.US_ASCII));
        }
    }
    
    public @Hash byte[] hashSha(@PlainText byte[] data) {
        return hashSha(null, data);
    }
    
    public @Hash byte[] hashSha(@Salt byte @Nullable [] seed, @PlainText byte[] data) {
        try {
            MessageDigest digest = (MessageDigest)this.sha256digest.clone();
            if (seed != null) digest.update(seed);
            return digest.digest(data);
        } catch (CloneNotSupportedException ex) {
            throw new RuntimeException(ex);
        }
    }

    public @Hash byte[] hashMd5(@PlainText byte[] data) {
        return hashMd5(null, data);
    }

    public @Hash byte[] hashMd5(@Salt byte @Nullable [] seed, @PlainText byte[] data) {
        try {
            MessageDigest digest = (MessageDigest)this.md5digest.clone();
            if (seed != null) digest.update(seed);
            return digest.digest(data);
        } catch (CloneNotSupportedException ex) {
            throw new RuntimeException(ex);
        }
    }
    
    public static @Hash byte[] hashShaStatic(@Salt byte @Nullable [] seed, @PlainText byte[] data) {
        try {
            MessageDigest digest = (MessageDigest)g_sha256digest.clone();
            if (seed != null) digest.update(seed);
            return digest.digest(data);
        } catch (CloneNotSupportedException ex) {
            throw new RuntimeException(ex);
        }
    }
    
    public @Hash String hashShaAndEncode(@PlainText String data) {
        return hashShaAndEncode(data.getBytes(Charsets.US_ASCII));
    }

    public @Hash String hashShaAndEncode(@Salt String seed, Iterable<@PlainText String> datas) {
        return hashShaAndEncode(Iterables.concat(Lists.newArrayList(seed), datas));
    }

    public @Hash String hashShaAndEncode(@Salt String seed, Iterable<@PlainText String> datas1, Iterable<@PlainText String> datas2) {
        return hashShaAndEncode(Iterables.concat(Lists.newArrayList(seed), Iterables.concat(datas1, datas2)));
    }

    public @Hash String hashShaAndEncode(Iterable<@PlainText String> datas) {
        try {
            MessageDigest digest = (MessageDigest)this.sha256digest.clone();
            for (String data : datas) {
                if (data == null) continue;
                digest.update(data.getBytes(Charsets.US_ASCII));
            }
            byte[] ret = digest.digest();
            return Base64.encodeBase64URLSafeString(ret);
        } catch (CloneNotSupportedException ex) {
            throw new RuntimeException(ex);
        }
    }
    
    public @Hash String hashShaAndEncode(@Salt byte @Nullable [] seed, @PlainText byte[] data) {
        return Base64.encodeBase64URLSafeString(hashSha(seed, data));
    }

    public @Hash String hashShaAndEncode(@PlainText byte[] data) {
        return Base64.encodeBase64URLSafeString(hashSha(data));
    }

    public @Hash String hashMd5AndEncode(@Salt byte @Nullable [] seed, @PlainText byte[] data) {
        return Base64.encodeBase64URLSafeString(hashMd5(seed, data));
    }

    public @Hash String hashMd5AndEncode(@PlainText byte[] data) {
        return Base64.encodeBase64URLSafeString(hashMd5(data));
    }
    
    public byte[] extractKey(CipherParameters key) {
        if (key instanceof NTRUEncryptionPublicKeyParameters) {
            return ((NTRUEncryptionPublicKeyParameters)key).getEncoded();
        }
        if (key instanceof NTRUEncryptionPrivateKeyParameters) {
            return ((NTRUEncryptionPrivateKeyParameters)key).getEncoded();
        }
        if (key instanceof NTRUSigningPublicKeyParameters) {
            return ((NTRUSigningPublicKeyParameters)key).getEncoded();
        }
        if (key instanceof NTRUSigningPrivateKeyParameters) {
            try {
                return ((NTRUSigningPrivateKeyParameters)key).getEncoded();
            } catch (IOException ex) {
                throw new RuntimeException(ex);
            }
        }
        if (key instanceof NHPublicKeyParameters) {
            return ((NHPublicKeyParameters) key).getPubData();
        }
        if (key instanceof NHPrivateKeyParameters) {
            short[] secData = ((NHPrivateKeyParameters) key).getSecData();
            byte[] privateKey = new byte[secData.length * 2];
            ByteBuffer privateBB = ByteBuffer.wrap(privateKey);
            for (int index = 0; index < secData.length; index++) {
                privateBB.putShort(secData[index]);
            }
            return privateKey;
        }
        if (key instanceof QTESLAPublicKeyParameters) {
            return ((QTESLAPublicKeyParameters) key).getPublicData();
        }
        if (key instanceof QTESLAPrivateKeyParameters) {
            return ((QTESLAPrivateKeyParameters) key).getSecret();
        }
        if (key instanceof XMSSMTPublicKeyParameters) {
            return XmssKeySerializer.serialize((XMSSMTPublicKeyParameters) key);
        }
        if (key instanceof XMSSMTPrivateKeyParameters) {
            return XmssKeySerializer.serialize((XMSSMTPrivateKeyParameters) key);
        }
        if (key instanceof RainbowPublicKeyParameters) {
            return RainbowKeySerializer.serialize((RainbowPublicKeyParameters)key);
        }
        if (key instanceof RainbowPrivateKeyParameters) {
            return RainbowKeySerializer.serialize((RainbowPrivateKeyParameters)key);
        }
        throw new RuntimeException("Unable to extract the key as it is an unknown type");
    }
    
    public @Hash String extractKeyHash(CipherParameters key) {
        if (key instanceof NTRUEncryptionPublicKeyParameters) {
            return this.hashShaAndEncode(((NTRUEncryptionPublicKeyParameters)key).getEncoded());
        }
        if (key instanceof NTRUEncryptionPrivateKeyParameters) {
            return this.hashShaAndEncode(((NTRUEncryptionPrivateKeyParameters)key).getEncoded());
        }
        if (key instanceof NTRUSigningPublicKeyParameters) {
            return this.hashShaAndEncode(((NTRUSigningPublicKeyParameters)key).getEncoded());
        }
        if (key instanceof NTRUSigningPrivateKeyParameters) {
            try {
                return this.hashShaAndEncode(this.hashShaAndEncode(((NTRUSigningPrivateKeyParameters)key).getEncoded()));
            } catch (IOException ex) {
                throw new RuntimeException(ex);
            }
        }
        if (key instanceof NHPublicKeyParameters) {
            return this.hashShaAndEncode(((NHPublicKeyParameters) key).getPubData());
        }
        if (key instanceof NHPrivateKeyParameters) {
            short[] secData = ((NHPrivateKeyParameters) key).getSecData();
            byte[] privateKey = new byte[secData.length * 2];
            ByteBuffer privateBB = ByteBuffer.wrap(privateKey);
            for (int index = 0; index < secData.length; index++) {
                privateBB.putShort(secData[index]);
            }
            return this.hashShaAndEncode(privateKey);
        }
        if (key instanceof QTESLAPublicKeyParameters) {
            return this.hashShaAndEncode(((QTESLAPublicKeyParameters) key).getPublicData());
        }
        if (key instanceof XMSSMTPublicKeyParameters) {
            return this.hashShaAndEncode(XmssKeySerializer.serialize((XMSSMTPublicKeyParameters) key));
        }
        if (key instanceof XMSSMTPrivateKeyParameters) {
            return this.hashShaAndEncode(XmssKeySerializer.serialize((XMSSMTPrivateKeyParameters) key));
        }
        if (key instanceof RainbowPublicKeyParameters) {
            return this.hashShaAndEncode(RainbowKeySerializer.serialize((RainbowPublicKeyParameters)key));
        }
        if (key instanceof RainbowPrivateKeyParameters) {
            return this.hashShaAndEncode(RainbowKeySerializer.serialize((RainbowPrivateKeyParameters)key));
        }
        throw new RuntimeException("Unable to extract the key as it is an unknown type");
    }

    public KeyPairBytes extractKey (AsymmetricCipherKeyPair pair) {
        return new KeyPairBytes(extractKey(pair.getPrivate()), extractKey(pair.getPublic()));
    }
    
    public MessagePrivateKeyDto getTrustOfPublicRead() {
        MessagePrivateKeyDto ret = this.trustOfPublicRead;
        if (ret == null) {
            ret = genEncryptKeyFromSeed(128, config.getDefaultEncryptTypes(), "public");
            //key = new MessagePrivateKeyDto("hCtNNY27gTrDwo2k1w_nm-28B_0u0Z8_lJYSqdmlRzpxb1Ke194tDZWyNEUR8uchT89qg_R1erx9CAyHFMYgAS2Gs5xfRy_37N2JmtR43HmEVDwcoytHjahdZGNYDIEzrSPhJuAb62unOwNjtS0LF9vkXR5akiyaxz7S21sKCitYwonYjGnODaf4axN6H6n_jhhHIHsGORK_o-Giq7FKZNJhoVfyEaNZPsHkG763cKKSKzkvHHVt7EONjW1OjFT6O5E0gNtiGDKQRquJBtWQUlsosDTaXCQWedj6HzBKsXQZjT_XL5QDSsUHIfTN4oiPqiNHREtjUuWMPa1GsOwhPSDRYpcsscBcD67gKRPeuk4_LfqwPk77ibEdbbP4g1FJhn8eaIGpXWTMFWG5Y_z8PfzS98K46Rj_dkHctVen3lHP_MiitAiUp4FtMdBl_FCHhpKFtoU0mriEUyjm1vLxxmgMuDVxb2Szo3Lm3Rgjq2ZSQBj9Sea-GuqBwc_7uBkqZY-vb72FqQ54jy0-CP73Ij4uJ_uH2g93pJDzSfxPtmsZOp7Rs5pYT03gWr018llG4D4Xtsm-2xP_IONLasoJHTrkkg9XPvmxZSQ8_AUSLZfoGRjWxKrYS1qZqCoZ9zYf_x1UtQEpDFjs__Zo9JONKMieTTskykXv-SwSIiyA6EUbvBTN4-VFVZNmc8zCkBDRRH2jZZUCMbYGkuMXEO_aIM2YwYpRROUj48p7zo8uYlnB82YHvhb6czGWew-RSfNeMeE1vX2Z9qoVQRPgj-5dKbnG2Xbkifmjj4h4Aw", "hCtNNY27gTrDwo2k1w_nm-28B_0u0Z8_lJYSqdmlRzpxb1Ke194tDZWyNEUR8uchT89qg_R1erx9CAyHFMYgAS2Gs5xfRy_37N2JmtR43HmEVDwcoytHjahdZGNYDIEzrSPhJuAb62unOwNjtS0LF9vkXR5akiyaxz7S21sKCitYwonYjGnODaf4axN6H6n_jhhHIHsGORK_o-Giq7FKZNJhoVfyEaNZPsHkG763cKKSKzkvHHVt7EONjW1OjFT6O5E0gNtiGDKQRquJBtWQUlsosDTaXCQWedj6HzBKsXQZjT_XL5QDSsUHIfTN4oiPqiNHREtjUuWMPa1GsOwhPSDRYpcsscBcD67gKRPeuk4_LfqwPk77ibEdbbP4g1FJhn8eaIGpXWTMFWG5Y_z8PfzS98K46Rj_dkHctVen3lHP_MiitAiUp4FtMdBl_FCHhpKFtoU0mriEUyjm1vLxxmgMuDVxb2Szo3Lm3Rgjq2ZSQBj9Sea-GuqBwc_7uBkqZY-vb72FqQ54jy0-CP73Ij4uJ_uH2g93pJDzSfxPtmsZOp7Rs5pYT03gWr018llG4D4Xtsm-2xP_IONLasoJHTrkkg9XPvmxZSQ8_AUSLZfoGRjWxKrYS1qZqCoZ9zYf_x1UtQEpDFjs__Zo9JONKMieTTskykXv-SwSIiyA6EUbvBTN4-VFVZNmc8zCkBDRRH2jZZUCMbYGkuMXEO_aIM2YwYpRROUj48p7zo8uYlnB82YHvhb6czGWew-RSfNeMeE1vX2Z9qoVQRPgj-5dKbnG2Xbkifmjj4h4A35nyKJ3ikeM8yUi_FlKfk_c3f8Tacpp7F8UZUunoUF2VDvYohoTyU6FrHBK-PqRIKU-4HBkrR2LF6Y2zyABrr3C5axkSVArak7ofFERtX0shq9aj4OmCg");
            ret.setAlias("public");
            ret.getPrivateParts().immutalize();
            ret.getPublicParts().immutalize();
            this.trustOfPublicRead = ret;
        }
        return ret;
    }
    
    public MessagePrivateKeyDto getTrustOfPublicWrite() {
        MessagePrivateKeyDto ret = this.trustOfPublicWrite;
        if (ret == null) {
            ret = genSignKeyFromSeed(64, config.getDefaultSigningTypes(), "public");
            //key = new MessagePrivateKeyDto("rz39v_ev9aFHHJrhE0bn7RONg_RqfGNDXpARYuja8yHO2vf4npuodKpgMApzJW73V0-giMMXyweuYTP3fDtrrdQ_p-3hhAK91wqharZDf18PiU1HOzjFCAWSyQF6eDMzpAwoSUk1_sfL2nUTqF5s_oMlPkHcClBABvm0S3fKvJQC-HLPDpFFaCnsfStu-8ytyx_gjPnBSuGnL1qz5w", "AM232z_XLRsxcxJsNsjcDHJtj-Su62y7jTTn_QE4eFAA6ctcftImbHfTm04nfAmf5EhYcadcPzuwIdRZagyBOADleiEpAXtf4YqQnDX42scZvELRLoEjpofzo2Q5ncLKAOLkz9iZc3oS6PQpS8AZbEcrVq8qhSh_8MjpwYdDpG6vPf2_96_1oUccmuETRuftE42D9Gp8Y0NekBFi6NrzIc7a9_iem6h0qmAwCnMlbvdXT6CIwxfLB65hM_d8O2ut1D-n7eGEAr3XCqFqtkN_Xw-JTUc7OMUIBZLJAXp4MzOkDChJSTX-x8vadROoXmz-gyU-QdwKUEAG-bRLd8q8lAL4cs8OkUVoKex9K277zK3LH-CM-cFK4acvWrPnrz39v_ev9aFHHJrhE0bn7RONg_RqfGNDXpARYuja8yHO2vf4npuodKpgMApzJW73V0-giMMXyweuYTP3fDtrrdQ_p-3hhAK91wqharZDf18PiU1HOzjFCAWSyQF6eDMzpAwoSUk1_sfL2nUTqF5s_oMlPkHcClBABvm0S3fKvJQC-HLPDpFFaCnsfStu-8ytyx_gjPnBSuGnL1qz5w");
            ret.setAlias("public");
            this.trustOfPublicWrite = ret;
        }
        return ret;
    }
    
    /**
     * Creates a new password salt and returns it to the caller
     */
    public @Salt String generateSalt() {
        
        String ret = this.genSaltQueue.poll();
        this.moreKeys();
        if (ret != null) return ret;
        
        return new BigInteger(320, srandom).toString(16).toUpperCase();
    }

    /**
     * Creates a new password salt and returns it to the caller
     */
    public @Secret String generateSecret16(int numBits) {
        byte[] bytes = new byte[numBits/8];
        for (int n = 0; n < bytes.length; n++) {
            bytes[n] = (byte)srandom.nextInt();
        }
        
        StringBuilder sb = new StringBuilder(bytes.length * 2);
        for (byte b : bytes)
           sb.append(String.format("%02X", b));
        return sb.toString();
    }

    /**
     * Creates a new password salt and returns it to the caller
     */
    public @Secret String generateSecret64() {
        return generateSecret64(config.getDefaultAesStrength());
    }

    /**
     * Creates a new password salt and returns it to the caller
     */
    public @Secret String generateSecret64(int numBits) {
        if (numBits == 128) {
            String ret = this.genAes128Queue.poll();
            this.moreKeys();
            if (ret != null) return ret;
        } else if (numBits == 256) {
            String ret = this.genAes256Queue.poll();
            this.moreKeys();
            if (ret != null) return ret;
        } else if (numBits == 512) {
            String ret = this.genAes512Queue.poll();
            this.moreKeys();
            if (ret != null) return ret;
        }
        
        return generateSecret64Now(numBits);
    }

    /**
     * Creates a new password salt and returns it to the caller
     */
    public @Secret String generateSecret64Now(int numBits) {
        byte[] bytes = new byte[numBits/8];
        for (int n = 0; n < bytes.length; n++) {
            bytes[n] = (byte)srandom.nextInt();
        }
        return Base64.encodeBase64URLSafeString(bytes);
    }

    /**
     * Encrypts a string using a supplied key
     */
    public @Secret String encryptString(@Secret String encryptionKey, @Salt String iv, @PlainText String data) {
        try {
            // Build the key bytes
            byte[] keyBytes = DatatypeConverter.parseHexBinary(encryptionKey);
            byte[] ivBytes = DatatypeConverter.parseHexBinary(iv);
            byte[] input = data.getBytes("UTF-8");

            // wrap key data in Key/IV specs to pass to cipher
            SecretKeySpec key = new SecretKeySpec(keyBytes, "AES");
            IvParameterSpec ivSpec = new IvParameterSpec(ivBytes);

            // create the cipher with the algorithm you choose
            // see javadoc for Cipher class for more info, e.g.
            Cipher cipher = Encryptor.getAesCipherCbc();

            // Encrypt the string
            cipher.init(Cipher.ENCRYPT_MODE, key, ivSpec);
            byte[] encrypted = new byte[cipher.getOutputSize(input.length)];
            int enc_len = cipher.update(input, 0, input.length, encrypted, 0);
            enc_len += cipher.doFinal(encrypted, enc_len);

            // Return an encoded string of the data
            return Base64.encodeBase64URLSafeString(encrypted);
        } catch (InvalidKeyException | InvalidAlgorithmParameterException | ShortBufferException | IllegalBlockSizeException | BadPaddingException | UnsupportedEncodingException ex) {
            throw new RuntimeException("Problem encrypting encryption data:'" + data + "', using key:'" + encryptionKey + "' and nounce:'" + iv + "'", ex);
        }
    }

    /**
     * Decrypts a string using a supplied key
     */
    public @PlainText String decryptString(@Secret String encryptionKey, @Salt String iv, @Secret String encryptedData) {
        try {
            // Build the key bytes
            byte[] keyBytes = DatatypeConverter.parseHexBinary(encryptionKey);
            byte[] ivBytes = DatatypeConverter.parseHexBinary(iv);
            byte[] input = Base64.decodeBase64(encryptedData);
            int enc_len = input.length;

            // wrap key data in Key/IV specs to pass to cipher
            SecretKeySpec key = new SecretKeySpec(keyBytes, "AES");
            IvParameterSpec ivSpec = new IvParameterSpec(ivBytes);

            // create the cipher with the algorithm you choose
            // see javadoc for Cipher class for more info, e.g.
            Cipher cipher = Encryptor.getAesCipherCbc();

            // Decrypt the string
            cipher.init(Cipher.DECRYPT_MODE, key, ivSpec);
            byte[] decrypted = new byte[cipher.getOutputSize(enc_len)];
            int dec_len = cipher.update(input, 0, enc_len, decrypted, 0);
            dec_len += cipher.doFinal(decrypted, dec_len);

            // Convert the data back to string
            return new String(decrypted, "UTF-8");
        } catch (InvalidKeyException | InvalidAlgorithmParameterException | ShortBufferException | IllegalBlockSizeException | BadPaddingException | UnsupportedEncodingException ex) {
            throw new RuntimeException("Problem decrypting encryption data:'" + encryptedData + "', using key:'" + encryptionKey + "' and nounce:'" + iv + "'", ex);
        }
    }
    
    public @Hash String getPublicKeyHash(MessagePublicKey key)
    {
        @Hash String hash = key.hash();
        if (hash == null) {
            throw new RuntimeException("Public key does not have a hash attached.");
        }
        return hash;
    }
    
    public @Hash String getPublicKeyHash(MessagePublicKeyDto key)
    {
        @Hash String ret = key.getPublicKeyHash();
        if (ret == null) {
            throw new RuntimeException("Public key has no hash attached.");
        }
        return ret;
    }
    
    public @Hash String getPublicKeyHash(MessagePrivateKey key)
    {
        MessagePublicKey publicKey = key.publicKey();
        if (publicKey == null) {
            throw new RuntimeException("Pirvate key does not no public key attached.");
        }
        return this.getPublicKeyHash(publicKey);
    }
    
    public @Alias String getAlias(MessagePrivateKey key)
    {
        MessagePublicKey publicKey = key.publicKey();
        if (publicKey == null) {
            throw new RuntimeException("Private key does not no public key attached.");
        }
        return getAlias(publicKey);
    }
    
    public @Alias String getAlias(MessagePublicKey key)
    {
        @Alias String alias = key.alias();
        if (alias == null) return this.getPublicKeyHash(key);
        return alias;
    }

    public @Alias String getAlias(IPartitionKey partitionKey, MessagePublicKeyDto key)
    {
        @Hash String hash = key.getPublicKeyHash();
        @Alias String ret = key.getAlias();
        if (ret == null && hash != null) {
            IAteIO io = CDI.current().select(IAteIO.class, new AnnotationLiteral<BackendStorageSystem>(){}).get();
            MessagePublicKeyDto aliasKey = io.publicKeyOrNull(partitionKey, hash);
            if (aliasKey != null) ret = aliasKey.getAlias();
        }

        if (ret == null) ret = key.getPublicKeyHash();
        if (ret == null) {
            throw new RuntimeException("Private key has no alias.");
        }
        return ret;
    }

    public MessagePublicKey getPublicKey(MessagePrivateKey key)
    {
        MessagePublicKey publicKey = key.publicKey();
        if (publicKey == null) {
            throw new RuntimeException("Private key does not no public key attached.");
        }
        return publicKey;
    }
    
    public MessagePublicKeyDto getPublicKey(MessagePrivateKeyDto key)
    {
        return new MessagePublicKeyDto(key);
    }
    
    public MessagePublicKeyDto createPublicKeyWithAlias(MessagePublicKeyDto key, @Alias String alias)
    {
        MessagePublicKeyDto ret = new MessagePublicKeyDto(key);
        ret.setAlias(alias);
        return ret;
    }
    
    public MessagePrivateKeyDto createPrivateKeyWithAlias(MessagePrivateKeyDto key, @Alias String alias)
    {
        MessagePrivateKeyDto ret = new MessagePrivateKeyDto(key);
        ret.setAlias(alias);
        return ret;
    }

    public String serializePublicKey64(MessagePublicKeyDto key) {
        byte[] data = serializePublicKey(key);
        return Base64.encodeBase64URLSafeString(data);
    }

    public String serializePrivateKey64(MessagePrivateKeyDto key) {
        byte[] data = serializePrivateKey(key);
        return Base64.encodeBase64URLSafeString(data);
    }

    public MessagePublicKeyDto deserializePublicKey64(String data64) {
        return deserializePublicKey64WithAlias(data64, null);
    }

    public MessagePublicKeyDto deserializePrivateKey64(String data64) {
        return deserializePrivateKey64WithAlias(data64, null);
    }

    public MessagePublicKeyDto deserializePublicKey64WithAlias(String data64, @Nullable @Alias String alias) {
        byte[] data = Base64.decodeBase64(data64);
        return deserializePublicKeyWithAlias(data, alias);
    }

    public MessagePublicKeyDto deserializePrivateKey64WithAlias(String data64, @Nullable @Alias String alias) {
        byte[] data = Base64.decodeBase64(data64);
        return deserializePrivateKeyWithAlias(data, alias);
    }

    public byte[] serializePublicKey(MessagePublicKeyDto key) {
        ByteBuffer bb = key.createFlatBuffer().getByteBuffer().duplicate();
        byte[] ret = new byte[bb.remaining()];
        bb.get(ret);
        return ret;
    }

    public byte[] serializePrivateKey(MessagePrivateKeyDto key) {
        ByteBuffer bb = key.createPrivateKeyFlatBuffer().getByteBuffer().duplicate();
        byte[] ret = new byte[bb.remaining()];
        bb.get(ret);
        return ret;
    }

    public MessagePublicKeyDto deserializePublicKey(byte[] data) {
        return deserializePublicKeyWithAlias(data, null);
    }

    public MessagePublicKeyDto deserializePublicKeyWithAlias(byte[] data, @Nullable @Alias String alias)
    {
        ByteBuffer bb = ByteBuffer.wrap(data);
        MessagePublicKeyDto ret = new MessagePublicKeyDto(MessagePublicKey.getRootAsMessagePublicKey(bb));
        if (alias != null) {
            ret.setAlias(alias);
        }
        return ret;
    }

    public MessagePrivateKeyDto deserializePrivateKey(byte[] data) {
        return deserializePrivateKeyWithAlias(data, null);
    }

    public MessagePrivateKeyDto deserializePrivateKeyWithAlias(byte[] data, @Nullable @Alias String alias)
    {
        ByteBuffer bb = ByteBuffer.wrap(data);
        MessagePrivateKeyDto ret = new MessagePrivateKeyDto(MessagePrivateKey.getRootAsMessagePrivateKey(bb));
        if (alias != null) {
            ret.setAlias(alias);
        }
        return ret;
    }
}
