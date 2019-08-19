/*
 * Copyright 2018 John.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
package com.tokera.ate.test.encryptor;

import com.google.common.collect.Lists;
import com.tokera.ate.BootstrapConfig;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.dto.msg.MessageKeyPartDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.security.Encryptor;
import org.apache.commons.codec.binary.Base64;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;
import org.bouncycastle.crypto.InvalidCipherTextException;

import java.io.IOException;

/**
 *
 * @author John
 */
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public class CryptoTests {

    private final static Encryptor encryptor = new Encryptor();

    @SuppressWarnings("deprecation")
    @BeforeAll
    public static void init() {
        encryptor.init();
        encryptor.setBootstrapConfig(new BootstrapConfig());
    }

    private void testSeededSigningKeyInternal(int keysize)
    {
        MessagePrivateKeyDto key1 = encryptor.genSignKeyFromSeed(keysize, "samekey");
        Assertions.assertEquals(key1.getPrivateParts().size(), key1.getPublicParts().size());

        MessagePrivateKeyDto key2 = encryptor.genSignKeyFromSeed(keysize, "samekey");

        Assertions.assertEquals(key1, key2);
        Assertions.assertEquals(key1.getPrivateParts().size(), key2.getPrivateParts().size());
        Assertions.assertEquals(key1.getPublicParts().size(), key2.getPublicParts().size());
        for (int n = 0; n < key1.getPrivateParts().size(); n++) {
            MessageKeyPartDto part1 = key1.getPrivateParts().get(n);
            MessageKeyPartDto part2 = key1.getPrivateParts().get(n);
            Assertions.assertEquals(part1.getType(), part2.getType());
            Assertions.assertEquals(part1.getSize(), part2.getSize());
            Assertions.assertEquals(part1.getKey64(), part2.getKey64());

            part1 = key1.getPublicParts().get(n);
            part2 = key1.getPublicParts().get(n);
            Assertions.assertEquals(part1.getType(), part2.getType());
            Assertions.assertEquals(part1.getSize(), part2.getSize());
            Assertions.assertEquals(part1.getKey64(), part2.getKey64());
        }
    }

    private void testSeededEncryptionKeyInternal(int keysize)
    {
        MessagePrivateKeyDto key1 = encryptor.genEncryptKeyFromSeed(keysize, "samekey");
        MessagePrivateKeyDto key2 = encryptor.genEncryptKeyFromSeed(keysize, "samekey");

        Assertions.assertEquals(key1, key2);
        Assertions.assertEquals(key1.getPrivateParts().size(), key2.getPrivateParts().size());
        Assertions.assertEquals(key1.getPublicParts().size(), key2.getPublicParts().size());
        for (int n = 0; n < key1.getPrivateParts().size(); n++) {
            MessageKeyPartDto part1 = key1.getPrivateParts().get(n);
            MessageKeyPartDto part2 = key1.getPrivateParts().get(n);
            Assertions.assertEquals(part1.getType(), part2.getType());
            Assertions.assertEquals(part1.getSize(), part2.getSize());
            Assertions.assertEquals(part1.getKey64(), part2.getKey64());

            part1 = key1.getPublicParts().get(n);
            part2 = key1.getPublicParts().get(n);
            Assertions.assertEquals(part1.getType(), part2.getType());
            Assertions.assertEquals(part1.getSize(), part2.getSize());
            Assertions.assertEquals(part1.getKey64(), part2.getKey64());
        }
    }

    @Test
    public void testSeededSigningKey64()
    {
        testSeededSigningKeyInternal(64);
    }

    @Test
    public void testSeededSigningKey128()
    {
        testSeededSigningKeyInternal(128);
    }

    @Test
    public void testSeededSigningKey256()
    {
        testSeededSigningKeyInternal(256);
    }

    @Test
    public void testSeededSigningKey512()
    {
        testSeededSigningKeyInternal(512);
    }

    @Test
    public void testSeededEncryptionKey128()
    {
        testSeededEncryptionKeyInternal(128);
    }

    @Test
    public void testSeededEncryptionKey256()
    {
        testSeededEncryptionKeyInternal(256);
    }

    @Test
    public void testSeededEncryptionKey512()
    {
        testSeededEncryptionKeyInternal(512);
    }

    public void testSign(int keySize, @Nullable String _seed)
    {
        MessagePrivateKeyDto key1;
        MessagePrivateKeyDto key2;

        String seed = _seed;
        if (seed != null) {
            key1 = encryptor.genSignKeyFromSeed(keySize, seed);
        } else {
            key1 = encryptor.genSignKey(keySize);
        }

        String plain = "test";
        byte[] digest = encryptor.hashSha(null, plain.getBytes());

        byte[] sig = encryptor.sign(key1, digest);

        if (seed != null) {
            key2 = encryptor.genSignKeyFromSeed(keySize, seed);
        } else {
            key2 = key1;
        }

        boolean isValid = encryptor.verify(key2, digest, sig);

        Assertions.assertTrue(isValid);
    }

    public void testEncrypt(int keySize, @Nullable String _seed) throws IOException, InvalidCipherTextException
    {
        MessagePrivateKeyDto key1;
        MessagePrivateKeyDto key2;

        String seed = _seed;
        if (seed != null) {
            key1 = encryptor.genEncryptKeyFromSeed(keySize, seed);
        } else {
            key1 = encryptor.genEncryptKey(keySize);
        }

        Iterable<Integer> bitTests = Lists.newArrayList(32, 64, 128, 256, 512);
        for (int bits : bitTests) {
            String plain = encryptor.generateSecret64(bits);
            byte[] plainBytes = Base64.decodeBase64(plain);

            byte[] enc = encryptor.encrypt(key1, plainBytes);

            if (seed != null) {
                key2 = encryptor.genEncryptKeyFromSeed(keySize, seed);
            } else {
                key2 = key1;
            }

            byte[] plainBytes2 = encryptor.decrypt(key2, enc);
            String plain2 = Base64.encodeBase64URLSafeString(plainBytes2);

            Assertions.assertEquals(plain, plain2);
        }
    }

    @Test
    public void testSign64() {
        testSign(64, null);
    }

    @Test
    public void testSign128() {
        testSign(128, null);
    }

    @Test
    public void testSign256() {
        testSign(256, null);
    }

    @Test
    public void testSign512() {
        testSign(512, null);
    }

    @Test
    public void testEncrypt128() throws IOException, InvalidCipherTextException {
        testEncrypt(128, null);
    }

    @Test
    public void testEncrypt256() throws IOException, InvalidCipherTextException {
        testEncrypt(256, null);
    }

    @Test
    public void testEncrypt512() throws IOException, InvalidCipherTextException {
        testEncrypt(512, null);
    }

    @Test
    public void testSign64Public() {
        testSign(64, "public");
    }

    @Test
    public void testSign128Public() {
        testSign(128, "public");
    }

    @Test
    public void testSign256Public() {
        testSign(256, "public");
    }

    @Test
    public void testSign512Public() {
        testSign(512, "public");
    }

    @Test
    public void testEncrypt128Public() throws IOException, InvalidCipherTextException {
        testEncrypt(128, "public");
    }

    @Test
    public void testEncrypt256Public() throws IOException, InvalidCipherTextException {
        testEncrypt(256, "public");
    }

    @Test
    public void testEncrypt512Public() throws IOException, InvalidCipherTextException {
        testEncrypt(512, "public");
    }

    /*
    //@Test
    public void generateSignKeys() {
        for (int n = 0; n < 4; n++) {
            MessagePrivateKeyDto key = encryptor.genSignKey(64);
            //System.out.println(yamlDelegate.serializeObj(key));
        }
    }

    //@Test
    public void generateEncryptKeys() {
        for (int n = 0; n < 32; n++) {
            MessagePrivateKeyDto key = encryptor.genEncryptKey(128);
            //System.out.println(yamlDelegate.serializeObj(key));
        }
    }
    */

    @Test
    public void generateSignKeys() {
        for (int n = 0; n < 20; n++) {
            PrivateKeyWithSeedDto key = encryptor.genSignKeyAndSeed();
            System.out.println(AteDelegate.get().yaml.serializeObj(key));
        }
    }

    @Test
    public void generateEncryptKeys() {
        for (int n = 0; n < 20; n++) {
            PrivateKeyWithSeedDto key = encryptor.genEncryptKeyAndSeed();
            System.out.println(AteDelegate.get().yaml.serializeObj(key));
        }
    }
}
