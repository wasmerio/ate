package com.tokera.ate.security;

import com.google.common.base.Charsets;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.scopes.Startup;
import com.tokera.ate.io.api.IAteIO;
import com.tokera.ate.qualifiers.BackendStorageSystem;
import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.security.core.*;
import com.tokera.ate.units.*;
import com.tokera.ate.dao.msg.MessagePrivateKey;
import com.tokera.ate.dao.msg.MessagePublicKey;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;

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
import java.util.ArrayList;
import java.util.Arrays;
import java.util.concurrent.ConcurrentLinkedQueue;
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
import org.bouncycastle.crypto.*;
import org.bouncycastle.pqc.crypto.ExchangePair;
import org.bouncycastle.pqc.crypto.newhope.*;
import org.bouncycastle.pqc.crypto.ntru.*;
import org.bouncycastle.pqc.crypto.qtesla.*;
import org.bouncycastle.pqc.crypto.xmss.XMSSMTKeyGenerationParameters;
import org.bouncycastle.pqc.crypto.xmss.XMSSMTParameters;
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
    
    private final int ntruSignParams128thresholdPrivate = (1556 + 442) / 2;
    private final int ntruSignParams128thresholdPublic = (604 + 157) / 2;
    private final int ntruSignParams256thresholdPrivate = (2636 + 1556)/2;
    private final int ntruSignParams256thresholdPublic = (1022 + 604)/2;
    
    private final int ntruEncryptParams256thresholdPrivate = (1170 + 691) / 2;
    private final int ntruEncryptParams256thresholdPublic = (1022 + 604)/2;

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

    private NTRUSigningKeyGenerationParameters buildNtruSignParams64() {
        return new NTRUSigningKeyGenerationParameters(157, 256, 29, 1, NTRUSigningKeyGenerationParameters.BASIS_TYPE_TRANSPOSE, 0.38, 200, 80, false, false, NTRUSigningKeyGenerationParameters.KEY_GEN_ALG_RESULTANT, new SHA256Digest());
    }

    private NTRUSigningKeyGenerationParameters buildNtruSignParams128() {
        return new NTRUSigningKeyGenerationParameters(439, 2048, 146, 1, NTRUSigningKeyGenerationParameters.BASIS_TYPE_TRANSPOSE, 0.165, 490, 280, false, true, NTRUSigningKeyGenerationParameters.KEY_GEN_ALG_RESULTANT, new SHA256Digest());
    }

    private NTRUSigningKeyGenerationParameters buildNtruSignParams256() {
        return new NTRUSigningKeyGenerationParameters(743, 2048, 248, 1, NTRUSigningKeyGenerationParameters.BASIS_TYPE_TRANSPOSE, 0.127, 560, 360, true, false, NTRUSigningKeyGenerationParameters.KEY_GEN_ALG_RESULTANT, new SHA512Digest());
    }

    private NTRUEncryptionKeyGenerationParameters buildNtruEncryptParams128() {
        return new NTRUEncryptionKeyGenerationParameters(439, 2048, 146, 130, 128, 9, 32, 9, true, new byte[]{0, 7, 101}, true, false, new SHA256Digest());
    }

    private NTRUEncryptionKeyGenerationParameters buildNtruEncryptParams256() {
        return new NTRUEncryptionKeyGenerationParameters(743, 2048, 248, 220, 256, 10, 27, 14, true, new byte[]{0, 7, 105}, false, false, new SHA512Digest());
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
                genSign64Queue.add(this.genSignKeyNow(64));
                cntSign64++;
                didGen = true;
            }
            if (cntSign128 < c_KeyPreGen128 && cntSign128 < cap) {
                genSign128Queue.add(this.genSignKeyNow(128));
                cntSign128++;
                didGen = true;
            }
            if (cntSign256 < c_KeyPreGen256 && cntSign256 < cap) {
                genSign256Queue.add(this.genSignKeyNow(256));
                cntSign256++;
                didGen = true;
            }
            if (cntEncrypt128 < c_KeyPreGen128 && cntEncrypt128 < cap) {
                genEncrypt128Queue.add(this.genEncryptKeyNow(128));
                cntEncrypt128++;
                didGen = true;
            }
            if (cntEncrypt256 < c_KeyPreGen256 && cntEncrypt256 < cap) {
                genEncrypt256Queue.add(this.genEncryptKeyNow(256));
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

    @Deprecated
    public MessagePrivateKeyDto genSignKey(int keysize)
    {
        return genSignKey(keysize, null);
    }

    @Deprecated
    public MessagePrivateKeyDto genSignKey(int keysize, @Nullable @Alias String _alias)
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

        return genSignKeyNow(keysize, alias);
    }

    public MessagePrivateKeyDto genSignKeyNow(int keysize) {
        return genSignKeyNow(keysize, null);
    }

    public MessagePrivateKeyDto genSignKeyNow(int keysize, @Nullable @Alias String alias) {
        KeyPairBytes pair1 = genSignKeyQTeslaNow(keysize);
        KeyPairBytes pair2 = genSignKeyXmssNow(keysize);
        return extractKey(pair1, pair2, alias);
    }

    public MessagePrivateKeyDto genSignKeyFromSeed(int keysize, String seed) {
        return genSignKeyFromSeed(keysize, null);
    }

    public MessagePrivateKeyDto genSignKeyFromSeed(int keysize, String seed, @Nullable @Alias String alias) {
        PredictablyRandom random = new PredictablyRandom(seed);
        KeyPairBytes pair1 = genSignKeyQTeslaFromSeed(keysize, random);
        KeyPairBytes pair2 = genSignKeyXmssFromSeed(keysize, random);
        return extractKey(pair1, pair2, alias);
    }

    @Deprecated
    public KeyPairBytes genSignKeyNtruNow(int keysize)
    {
        return genSignKeyNtruNow(keysize, null); 
    }

    @Deprecated
    public KeyPairBytes genSignKeyNtruNow(int keysize, @Nullable @Alias String alias)
    {
        for (int n = 0; n < 8; n++) {
            SigningKeyPairGenerator keyGen = new SigningKeyPairGenerator();
            switch (keysize) {
                case 256:
                    keyGen.init(buildNtruSignParams256());
                    break;
                case 128:
                    keyGen.init(buildNtruSignParams128());
                    break;
                case 64:
                    keyGen.init(buildNtruSignParams64());
                    break;
                default:
                    throw new RuntimeException("Unknown NTRU key size(" + keysize + ")");
            }

            AsymmetricCipherKeyPair pair = keyGen.generateKeyPair(new UnPredictablyRandom());
            if (testSignNtru(pair) == false) {
                continue;
            }

            return extractKey(pair);
        }
        throw new RuntimeException("Failed to generate signing key");
    }

    @Deprecated
    public KeyPairBytes genSignKeyNtruFromSeed(int keysize, @Salt String seed)
    {
        return genSignKeyNtruFromSeed(keysize, seed, null);
    }

    @Deprecated
    public KeyPairBytes genSignKeyNtruFromSeed(int keysize, @Salt String seed, @Nullable @Alias String alias)
    {
        SigningKeyPairGenerator gen = new SigningKeyPairGenerator();
        switch (keysize) {
            case 256:
                gen.init(buildNtruSignParams256());
                break;
            case 128:
                gen.init(buildNtruSignParams128());
                break;
            case 64:
                gen.init(buildNtruSignParams64());
                break;
            default:
                throw new RuntimeException("Unknown NTRU key size(" + keysize + ")");
        }
        
        AsymmetricCipherKeyPair pair = gen.generateKeyPair(new PredictablyRandom(seed));
        if (testSignNtru(pair) == false) {
            throw new RuntimeException("Failed to generate signing key from seed");
        }
        return extractKey(pair);
    }

    @Deprecated
    private boolean testSignNtru(AsymmetricCipherKeyPair pair) {
        
        NTRUSigningPrivateKeyParameters privateKey = (NTRUSigningPrivateKeyParameters) pair.getPrivate();
        NTRUSigningPublicKeyParameters publicKey = (NTRUSigningPublicKeyParameters) pair.getPublic();
        String test = "thecatranupthewall";
                
        try {
            byte[] sig = this.signNtru(privateKey.getEncoded(), test.getBytes());
            if (this.verifyNtru(publicKey.getEncoded(), test.getBytes(), sig) == false) {
                return false;
            }
            return true;
        } catch (Throwable ex) {
            return false;
        }
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
        
        return genEncryptKeyNow(keysize);
    }

    public MessagePrivateKeyDto genEncryptKey(int keysize, @Nullable @Alias String _alias)
    {
        MessagePrivateKeyDto key = genEncryptKey(keysize);

        @Alias String alias = _alias;
        if (alias == null) return key;
        key.setAlias(alias);

        return key;
    }

    public MessagePrivateKeyDto genEncryptKeyNow(int keysize) {
        return genEncryptKeyNow(keysize, null);
    }

    public MessagePrivateKeyDto genEncryptKeyNow(int keysize, @Nullable @Alias String alias) {
        KeyPairBytes pair1 = genEncryptKeyNtruNow(keysize);
        KeyPairBytes pair2 = genEncryptKeyNewHopeNow(keysize);
        return extractKey(pair1, pair2, alias);
    }

    public MessagePrivateKeyDto genEncryptKeyFromSeed(int keysize, String seed) {
        return genEncryptKeyFromSeed(keysize, null);
    }

    public MessagePrivateKeyDto genEncryptKeyFromSeed(int keysize, String seed, @Nullable @Alias String alias) {
        PredictablyRandom random = new PredictablyRandom(seed);
        KeyPairBytes pair1 = genEncryptKeyNtruFromSeed(keysize, random);
        KeyPairBytes pair2 = genEncryptKeyNewHopeFromSeed(keysize, random);
        return extractKey(pair1, pair2, alias);
    }
    
    public KeyPairBytes genEncryptKeyNtruNow(int keysize)
    {
        for (int n = 0; n < 8; n++) {
            EncryptionKeyPairGenerator keyGen = new EncryptionKeyPairGenerator();
            switch (keysize) {
                case 256:
                    keyGen.init(buildNtruEncryptParams256());
                    break;
                case 128:
                    keyGen.init(buildNtruEncryptParams128());
                    break;
                default:
                    throw new RuntimeException("Unknown NTRU key size(" + keysize + ")");
            }

            AsymmetricCipherKeyPair pair = keyGen.generateKeyPair(new UnPredictablyRandom());
            if (testKey(pair) == false) {
                continue;
            }
            return extractKey(pair);
        }
        throw new RuntimeException("Failed to generate encryption key");
    }

    public KeyPairBytes genEncryptKeyNtruFromSeed(int keysize, @Secret String seed)
    {
        PredictablyRandom random = new PredictablyRandom(seed);
        return genEncryptKeyNtruFromSeed(keysize, random);
    }
    
    public KeyPairBytes genEncryptKeyNtruFromSeed(int keysize, PredictablyRandom random)
    {
        EncryptionKeyPairGenerator gen = new EncryptionKeyPairGenerator();
        switch (keysize) {
            case 256:
                gen.init(buildNtruEncryptParams256());
                break;
            case 128:
                gen.init(buildNtruEncryptParams128());
                break;
            default:
                throw new RuntimeException("Unknown NTRU key size(" + keysize + ")");
        }

        AsymmetricCipherKeyPair pair = gen.generateKeyPair(random);
        if (testKey(pair) == false) {
            throw new RuntimeException("Failed to generate encryption key from seed");
        }
        return extractKey(pair);
    }
    
    private boolean testKey(AsymmetricCipherKeyPair pair) {
        
        NTRUEncryptionPrivateKeyParameters privateKey = (NTRUEncryptionPrivateKeyParameters) pair.getPrivate();
        NTRUEncryptionPublicKeyParameters publicKey = (NTRUEncryptionPublicKeyParameters) pair.getPublic();

        for (int n = 0; n < 10; n++) {
            byte[] test = Base64.decodeBase64(this.generateSecret64(128));

            try {
                byte[] encBytes = this.encryptNtruWithPublic(publicKey.getEncoded(), test);
                byte[] plainBytes = this.decryptNtruWithPrivate(privateKey.getEncoded(), encBytes);
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
        PredictablyRandom random = new PredictablyRandom(seed);
        return genEncryptKeyNewHopeFromSeed(keysize, random);
    }

    public KeyPairBytes genEncryptKeyNewHopeFromSeed(int keysize, PredictablyRandom random)
    {
        NHKeyPairGeneratorPredictable gen = new NHKeyPairGeneratorPredictable();
        gen.init(random);
        return extractKey(gen.generateKeyPair());
    }

    public KeyPairBytes genEncryptKeyNewHopeNow(int keysize)
    {
        SecureRandom keyRandom = new SecureRandom();
        KeyGenerationParameters params = new KeyGenerationParameters(keyRandom, keysize);

        NHKeyPairGenerator gen = new NHKeyPairGenerator();
        gen.init(params);
        return extractKey(gen.generateKeyPair());
    }

    public @Secret byte[] encryptNewHopeWithPublic(@Secret byte[] publicKey, @PlainText byte[] data)
    {
        NHPublicKeyParameters params = new NHPublicKeyParameters(publicKey);
        ExchangePair exchangeSecret = new NHExchangePairGenerator(new SecureRandom()).generateExchange(params);
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

    public @PlainText byte[] decryptNewHopeWithPrivate(@Secret byte[] privateKey, @Secret byte[] data)
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
    
    public @Secret byte[] encryptNtruWithPublic(@Secret byte[] key, @PlainText byte[] data)
    {
        try {
            NTRUEncryptionKeyGenerationParameters ntruEncryptParams;
            if (key.length >= ntruEncryptParams256thresholdPublic) {
                ntruEncryptParams = buildNtruEncryptParams256();
            } else {
                ntruEncryptParams = buildNtruEncryptParams128();
            }
            
            NTRUEncryptionPublicKeyParameters priv = new NTRUEncryptionPublicKeyParameters(key, ntruEncryptParams.getEncryptionParameters());
            
            NTRUEngine engine = new NTRUEngine();
            engine.init(true, priv);

            return engine.processBlock(data, 0, data.length);
            
        } catch (InvalidCipherTextException ex) {
            throw new RuntimeException(ex);
        }
    }
    
    public @PlainText byte[] decryptNtruWithPrivate(@Secret byte[] key, @Secret byte[] data) throws IOException, InvalidCipherTextException
    {
        NTRUEncryptionKeyGenerationParameters ntruEncryptParams;
        if (key.length >= ntruEncryptParams256thresholdPrivate) {
            ntruEncryptParams = buildNtruEncryptParams256();
        } else {
            ntruEncryptParams = buildNtruEncryptParams128();
        }
        
        NTRUEncryptionPrivateKeyParameters priv = new NTRUEncryptionPrivateKeyParameters(key, ntruEncryptParams.getEncryptionParameters());

        NTRUEngine engine = new NTRUEngine();
        engine.init(false, priv);

        return engine.processBlock(data, 0, data.length);
    }

    public KeyPairBytes genSignKeyQTeslaNow(int keysize)
    {
        SecureRandom keyRandom = new SecureRandom();

        QTESLAKeyGenerationParameters params;
        switch (keysize) {
            case 512:
                params = new QTESLAKeyGenerationParameters(QTESLASecurityCategory.PROVABLY_SECURE_III, keyRandom);
                break;
            case 256:
                params = new QTESLAKeyGenerationParameters(QTESLASecurityCategory.HEURISTIC_III_SPEED, keyRandom);
                break;
            case 128:
            case 64:
                params = new QTESLAKeyGenerationParameters(QTESLASecurityCategory.HEURISTIC_I, keyRandom);
                break;
            default:
                throw new RuntimeException("Unknown GMSS key size(" + keysize + ")");
        }
        QTESLAKeyPairGenerator gen = new QTESLAKeyPairGenerator();
        gen.init(params);
        return extractKey(gen.generateKeyPair());
    }

    public @Signature byte[] signQTesla(@Secret byte[] privateKey, @Hash byte[] digest)
    {
        int securityCategory = QTESLASecurityCategory.HEURISTIC_I;
        if (privateKey.length > 2000) securityCategory = QTESLASecurityCategory.HEURISTIC_III_SPEED;
        if (privateKey.length > 8000) securityCategory = QTESLASecurityCategory.PROVABLY_SECURE_III;

        QTESLAPrivateKeyParameters params = new QTESLAPrivateKeyParameters(securityCategory, privateKey);

        QTESLASigner signer = new QTESLASigner();
        signer.init(true, params);
        return signer.generateSignature(digest);
    }

    public boolean verifyQTesla(@PEM byte[] publicKey, @Hash byte[] digest, @Signature byte[] sig)
    {
        int securityCategory = QTESLASecurityCategory.HEURISTIC_I;
        if (publicKey.length > 2500) securityCategory = QTESLASecurityCategory.HEURISTIC_III_SPEED;
        if (publicKey.length > 20000) securityCategory = QTESLASecurityCategory.PROVABLY_SECURE_III;

        QTESLAPublicKeyParameters params = new QTESLAPublicKeyParameters(securityCategory, publicKey);

        QTESLASigner signer = new QTESLASigner();
        signer.init(false, params);
        return signer.verifySignature(digest, sig);
    }

    public KeyPairBytes genSignKeyXmssMtNow(int keysize)
    {
        SecureRandom keyRandom = new SecureRandom();


        XMSSMTParameters params = new XMSSMTParameters();

        XMSSMTKeyGenerationParameters params = new XMSSMTKeyGenerationParameters();
        QTESLAKeyGenerationParameters params;
        switch (keysize) {
            case 512:
                params = new QTESLAKeyGenerationParameters(QTESLASecurityCategory.PROVABLY_SECURE_III, keyRandom);
                break;
            case 256:
                params = new QTESLAKeyGenerationParameters(QTESLASecurityCategory.HEURISTIC_III_SPEED, keyRandom);
                break;
            case 128:
            case 64:
                params = new QTESLAKeyGenerationParameters(QTESLASecurityCategory.HEURISTIC_I, keyRandom);
                break;
            default:
                throw new RuntimeException("Unknown GMSS key size(" + keysize + ")");
        }
        QTESLAKeyPairGenerator gen = new QTESLAKeyPairGenerator();
        gen.init(params);
        return extractKey(gen.generateKeyPair());
    }

    public @Signature byte[] signXmssMt(@Secret byte[] privateKey, @Hash byte[] digest)
    {
        int securityCategory = QTESLASecurityCategory.HEURISTIC_I;
        if (privateKey.length > 2000) securityCategory = QTESLASecurityCategory.HEURISTIC_III_SPEED;
        if (privateKey.length > 8000) securityCategory = QTESLASecurityCategory.PROVABLY_SECURE_III;

        QTESLAPrivateKeyParameters params = new QTESLAPrivateKeyParameters(securityCategory, privateKey);

        QTESLASigner signer = new QTESLASigner();
        signer.init(true, params);
        return signer.generateSignature(digest);
    }

    public boolean verifyXmssMt(@PEM byte[] publicKey, @Hash byte[] digest, @Signature byte[] sig)
    {
        int securityCategory = QTESLASecurityCategory.HEURISTIC_I;
        if (publicKey.length > 2500) securityCategory = QTESLASecurityCategory.HEURISTIC_III_SPEED;
        if (publicKey.length > 20000) securityCategory = QTESLASecurityCategory.PROVABLY_SECURE_III;

        QTESLAPublicKeyParameters params = new QTESLAPublicKeyParameters(securityCategory, publicKey);

        QTESLASigner signer = new QTESLASigner();
        signer.init(false, params);
        return signer.verifySignature(digest, sig);
    }

    @Deprecated
    @SuppressWarnings("deprecation")
    public @Signature byte[] signNtru(@Secret byte[] privateKey, @Hash byte[] digest)
    {
        try {
            NTRUSigningKeyGenerationParameters ntruSignParams;
            if (privateKey.length >= ntruSignParams256thresholdPrivate) {
                ntruSignParams = buildNtruSignParams256();
            } else if (privateKey.length >= ntruSignParams128thresholdPrivate) {
                ntruSignParams = buildNtruSignParams128();
            } else {
                ntruSignParams = buildNtruSignParams64();
            }
            
            NTRUSigningPrivateKeyParameters priv = new NTRUSigningPrivateKeyParameters(privateKey, ntruSignParams);
            NTRUSigner signer = new NTRUSigner(ntruSignParams.getSigningParameters());
            signer.init(true, priv);            
            signer.update(digest, 0, digest.length);
            
            return signer.generateSignature();
        } catch (IOException ex) {
            throw new RuntimeException(ex);
        }
    }

    @Deprecated
    @SuppressWarnings("deprecation")
    public boolean verifyNtru(@PEM byte[] publicKey, @Hash byte[] digest, @Signature byte[] sig)
    {
        NTRUSigningKeyGenerationParameters ntruSignParams;
        if (publicKey.length >= ntruSignParams256thresholdPublic) {
            ntruSignParams = buildNtruSignParams256();
        } else if (publicKey.length >= ntruSignParams128thresholdPublic) {
            ntruSignParams = buildNtruSignParams128();
        } else {
            ntruSignParams = buildNtruSignParams64();
        }
            
        NTRUSigningPublicKeyParameters pub = new NTRUSigningPublicKeyParameters(publicKey, ntruSignParams.getSigningParameters());
        NTRUSigner signer = new NTRUSigner(ntruSignParams.getSigningParameters());
        signer.init(false, pub);
        signer.update(digest, 0, digest.length);

        return signer.verifySignature(sig);
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
        throw new RuntimeException("Unable to extract the key as it is an unknown type");
    }

    public KeyPairBytes extractKey (AsymmetricCipherKeyPair pair) {
        return new KeyPairBytes(extractKey(pair.getPrivate()), extractKey(pair.getPublic());
    }

    public MessagePrivateKeyDto extractKey(KeyPairBytes pair1, KeyPairBytes pair2) {
        return extractKey(pair1, pair2, null);
    }

    public MessagePrivateKeyDto extractKey(KeyPairBytes pair1, KeyPairBytes pair2, @Nullable @Alias String alias) {
        return createPrivateKey(pair1.publicKey, pair2.publicKey, pair1.privateKey, pair2.privateKey, alias);
    }
    
    public MessagePrivateKeyDto extractKey(AsymmetricCipherKeyPair pair1, AsymmetricCipherKeyPair pair2) {
        return extractKey(pair1, pair2, null);
    }
    
    public MessagePrivateKeyDto extractKey(AsymmetricCipherKeyPair pair1, AsymmetricCipherKeyPair pair2, @Nullable @Alias String alias) {
        return createPrivateKey(extractKey(pair1.getPublic()), extractKey(pair2.getPublic()), extractKey(pair1.getPrivate()), extractKey(pair2.getPrivate()), alias);
    }
    
    public MessagePrivateKeyDto getTrustOfPublicRead() {
        MessagePrivateKeyDto ret = this.trustOfPublicRead;
        if (ret == null) {
            ret = genEncryptKeyFromSeed(128, "public", "public");
            //key = new MessagePrivateKeyDto("hCtNNY27gTrDwo2k1w_nm-28B_0u0Z8_lJYSqdmlRzpxb1Ke194tDZWyNEUR8uchT89qg_R1erx9CAyHFMYgAS2Gs5xfRy_37N2JmtR43HmEVDwcoytHjahdZGNYDIEzrSPhJuAb62unOwNjtS0LF9vkXR5akiyaxz7S21sKCitYwonYjGnODaf4axN6H6n_jhhHIHsGORK_o-Giq7FKZNJhoVfyEaNZPsHkG763cKKSKzkvHHVt7EONjW1OjFT6O5E0gNtiGDKQRquJBtWQUlsosDTaXCQWedj6HzBKsXQZjT_XL5QDSsUHIfTN4oiPqiNHREtjUuWMPa1GsOwhPSDRYpcsscBcD67gKRPeuk4_LfqwPk77ibEdbbP4g1FJhn8eaIGpXWTMFWG5Y_z8PfzS98K46Rj_dkHctVen3lHP_MiitAiUp4FtMdBl_FCHhpKFtoU0mriEUyjm1vLxxmgMuDVxb2Szo3Lm3Rgjq2ZSQBj9Sea-GuqBwc_7uBkqZY-vb72FqQ54jy0-CP73Ij4uJ_uH2g93pJDzSfxPtmsZOp7Rs5pYT03gWr018llG4D4Xtsm-2xP_IONLasoJHTrkkg9XPvmxZSQ8_AUSLZfoGRjWxKrYS1qZqCoZ9zYf_x1UtQEpDFjs__Zo9JONKMieTTskykXv-SwSIiyA6EUbvBTN4-VFVZNmc8zCkBDRRH2jZZUCMbYGkuMXEO_aIM2YwYpRROUj48p7zo8uYlnB82YHvhb6czGWew-RSfNeMeE1vX2Z9qoVQRPgj-5dKbnG2Xbkifmjj4h4Aw", "hCtNNY27gTrDwo2k1w_nm-28B_0u0Z8_lJYSqdmlRzpxb1Ke194tDZWyNEUR8uchT89qg_R1erx9CAyHFMYgAS2Gs5xfRy_37N2JmtR43HmEVDwcoytHjahdZGNYDIEzrSPhJuAb62unOwNjtS0LF9vkXR5akiyaxz7S21sKCitYwonYjGnODaf4axN6H6n_jhhHIHsGORK_o-Giq7FKZNJhoVfyEaNZPsHkG763cKKSKzkvHHVt7EONjW1OjFT6O5E0gNtiGDKQRquJBtWQUlsosDTaXCQWedj6HzBKsXQZjT_XL5QDSsUHIfTN4oiPqiNHREtjUuWMPa1GsOwhPSDRYpcsscBcD67gKRPeuk4_LfqwPk77ibEdbbP4g1FJhn8eaIGpXWTMFWG5Y_z8PfzS98K46Rj_dkHctVen3lHP_MiitAiUp4FtMdBl_FCHhpKFtoU0mriEUyjm1vLxxmgMuDVxb2Szo3Lm3Rgjq2ZSQBj9Sea-GuqBwc_7uBkqZY-vb72FqQ54jy0-CP73Ij4uJ_uH2g93pJDzSfxPtmsZOp7Rs5pYT03gWr018llG4D4Xtsm-2xP_IONLasoJHTrkkg9XPvmxZSQ8_AUSLZfoGRjWxKrYS1qZqCoZ9zYf_x1UtQEpDFjs__Zo9JONKMieTTskykXv-SwSIiyA6EUbvBTN4-VFVZNmc8zCkBDRRH2jZZUCMbYGkuMXEO_aIM2YwYpRROUj48p7zo8uYlnB82YHvhb6czGWew-RSfNeMeE1vX2Z9qoVQRPgj-5dKbnG2Xbkifmjj4h4A35nyKJ3ikeM8yUi_FlKfk_c3f8Tacpp7F8UZUunoUF2VDvYohoTyU6FrHBK-PqRIKU-4HBkrR2LF6Y2zyABrr3C5axkSVArak7ofFERtX0shq9aj4OmCg");
            ret.setAlias("public");
            this.trustOfPublicRead = ret;
        }
        return ret;
    }
    
    public MessagePrivateKeyDto getTrustOfPublicWrite() {
        MessagePrivateKeyDto ret = this.trustOfPublicWrite;
        if (ret == null) {
            ret = genSignKeyFromSeed(64, "public", "public");
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
        
        SecureRandom srandom = new SecureRandom();
        return new BigInteger(320, srandom).toString(16).toUpperCase();
    }

    /**
     * Creates a new password salt and returns it to the caller
     */
    public @Secret String generateSecret16(int numBits) {
        SecureRandom srandom = new SecureRandom();
        
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
        SecureRandom srandom = new SecureRandom();
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
        @Hash String hash = key.publicKeyHash();
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
    
    public MessagePublicKeyDto createPublicKey(@PlainText String publicKey1Base64, @PlainText String publicKey2Base64, @Alias String alias)
    {
        MessagePublicKeyDto ret = new MessagePublicKeyDto(publicKey1Base64, publicKey2Base64);
        ret.setAlias(alias);
        return ret;
    }
    
    public MessagePublicKeyDto createPublicKey(MessagePublicKeyDto key, @Alias String alias)
    {
        MessagePublicKeyDto ret = new MessagePublicKeyDto(key);
        ret.setAlias(alias);
        return ret;
    }
    
    public MessagePrivateKeyDto createPrivateKey(MessagePrivateKeyDto key, @Alias String alias)
    {
        MessagePrivateKeyDto ret = new MessagePrivateKeyDto(key);
        ret.setAlias(alias);
        return ret;
    }
    
    public MessagePrivateKeyDto createPrivateKey(@PEM byte[] publicKeyBytes1, @PEM byte[] publicKeyBytes2, @Secret byte[] privateKeyBytes1, @Secret byte[] privateKeyBytes2, @Nullable @Alias String _alias)
    {
        MessagePrivateKeyDto ret = new MessagePrivateKeyDto(publicKeyBytes1, publicKeyBytes2, privateKeyBytes1, privateKeyBytes2);

        @Alias String alias = _alias;
        if (alias != null) {
            ret.setAlias(alias);
        }
        return ret;
    }
    
    public MessagePrivateKeyDto createPrivateKey(@PEM String publicKey1Base64, @PEM String publicKey2Base64, @Secret String privateKey1Base64, @Secret String privateKey2Base64, @Alias String alias)
    {
        return createPrivateKey(Base64.decodeBase64(publicKey1Base64), Base64.decodeBase64(publicKey2Base64), Base64.decodeBase64(privateKey1Base64), Base64.decodeBase64(privateKey2Base64), alias);
    }
}
