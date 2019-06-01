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

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.delegates.YamlDelegate;
import com.tokera.ate.dto.msg.MessageKeyPartDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.security.Encryptor;
import com.tokera.ate.test.dao.MyAccount;
import com.tokera.ate.test.dao.MyThing;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.jboss.weld.bootstrap.spi.BeanDiscoveryMode;
import org.jboss.weld.environment.se.Weld;
import org.jboss.weld.junit5.WeldInitiator;
import org.jboss.weld.junit5.WeldJunit5Extension;
import org.jboss.weld.junit5.WeldSetup;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;
import org.junit.jupiter.api.extension.ExtendWith;
import org.bouncycastle.crypto.InvalidCipherTextException;

import javax.enterprise.context.RequestScoped;
import javax.inject.Inject;
import java.io.IOException;

/**
 *
 * @author John
 */
@ExtendWith(WeldJunit5Extension.class)
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public class NtruTests {

    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private Encryptor encryptor;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private YamlDelegate yamlDelegate;

    @WeldSetup
    public WeldInitiator weld = WeldInitiator
            .from(new Weld()
                    .setBeanDiscoveryMode(BeanDiscoveryMode.ANNOTATED)
                    .enableDiscovery()
                    .addBeanClass(MyAccount.class)
                    .addBeanClass(MyThing.class))
            .activate(RequestScoped.class)
            .build();

    public void testSign(int keySize, @Nullable String _seed)
    {
        MessagePrivateKeyDto key;
        String seed = _seed;
        if (seed != null) {
            key = encryptor.genSignKeyFromSeed(keySize, seed);
        } else {
            key = encryptor.genSignKey(keySize);
        }

        String plain = "test";
        byte[] digest = encryptor.hashSha(null, plain.getBytes());

        byte[] sig = encryptor.sign(key, digest);
        boolean isValid = encryptor.verify(key, digest, sig);
        Assertions.assertTrue(isValid);
    }

    public void testEncrypt(int keySize, @Nullable String _seed) throws IOException, InvalidCipherTextException
    {
        MessagePrivateKeyDto key;
        String seed = _seed;
        if (seed != null) {
            key = encryptor.genEncryptKey(keySize, seed);
        } else {
            key = encryptor.genEncryptKey(keySize);
        }

        String plain = "test";

        byte[] enc = encryptor.encrypt(key, plain.getBytes());
        byte[] plainBytes = encryptor.decrypt(key, enc);

        String test = new String(plainBytes);
        Assertions.assertEquals(plain, test);
    }

    @Test
    public void testSign64() {
        testSign(64, null);
    }

    //@Test
    public void testSign128() {
        testSign(128, null);
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
    public void testSign64Public() {
        testSign(64, "public");
    }

    //@Test
    public void testSign128Public() {
        testSign(128, "public");
    }

    @Test
    public void testEncrypt128Public() throws IOException, InvalidCipherTextException {
        testEncrypt(128, "public");
    }

    @Test
    public void testEncrypt256Public() throws IOException, InvalidCipherTextException {
        testEncrypt(256, "public");
    }

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
}
