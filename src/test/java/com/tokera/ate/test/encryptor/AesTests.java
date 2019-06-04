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

import com.google.common.base.Objects;
import com.tokera.ate.BootstrapConfig;
import com.tokera.ate.security.Encryptor;
import java.nio.ByteBuffer;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;

import org.apache.commons.codec.binary.Base64;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;

/**
 *
 * @author John
 */
@TestInstance(TestInstance.Lifecycle.PER_METHOD)
public class AesTests {

    private static Encryptor encryptor = new Encryptor();
    
    private String payload = "!dao.account\n" +
                             "description: Mocked account created for testing.\n" +
                             "domain: tokera.com\n" +
                             "encryptKey: XjE-mqynk_rvL59bVblxng\n" +
                             "...\n";

    @SuppressWarnings("deprecation")
    @BeforeAll
    public static void init() {
        encryptor.init();
        encryptor.setBootstrapConfig(new BootstrapConfig());
    }

    private void performTest()
    {
        String plain = payload + UUID.randomUUID();
        byte[] plainBytes = plain.getBytes();
        
        byte[] encryptKey = Base64.decodeBase64(encryptor.generateSecret64Now(128));
        byte[] encPayload = encryptor.encryptAes(encryptKey, plainBytes);
        
        byte[] decryptedPayload = encryptor.decryptAes(encryptKey, ByteBuffer.wrap(encPayload));
        Assertions.assertArrayEquals(plainBytes, decryptedPayload);
        
        String decryptedString = new String(decryptedPayload);
        Assertions.assertTrue(Objects.equal(plain, decryptedString), "Plain text is not equal");
    }
    
    @Test
    public void testEncrypt()
    {
        performTest();
    }
    
    @Test
    public void testParallelEncrypt() throws InterruptedException, ExecutionException, ExecutionException, ExecutionException {
        final ExecutorService threads = Executors.newFixedThreadPool(100);
        try
        {
            List<Future> futures = new ArrayList<>();
            for (int n = 0; n < 10000; n++) {
                Future future = threads.submit(new Runnable() {
                    public void run() {
                        performTest();
                    }
                });
                futures.add(future);
            }
            
            for (Future future : futures) {
                future.get();
            }
        } finally {
            threads.shutdown();
        }
    }
}
