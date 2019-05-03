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

import com.tokera.ate.delegates.YamlDelegate;
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
import org.spongycastle.crypto.InvalidCipherTextException;

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
            key = encryptor.genSignKeyNtruFromSeed(keySize, seed);
        } else {
            key = encryptor.genSignKeyNtru(keySize);
        }
        byte[] keyPrivate = key.getPrivateKeyBytes();
        byte[] keyPublic = key.getPublicKeyBytes();
        assert keyPrivate != null : "@AssumeAssertion(nullness): Must not be null";
        assert keyPublic != null : "@AssumeAssertion(nullness): Must not be null";

        String plain = "test";
        byte[] digest = encryptor.hashSha(null, plain.getBytes());

        byte[] sig = encryptor.signNtru(keyPrivate, digest);
        boolean isValid = encryptor.verifyNtru(keyPublic, digest, sig);
        Assertions.assertTrue(isValid);
    }

    public void testEncrypt(int keySize, @Nullable String _seed) throws IOException, InvalidCipherTextException
    {
        MessagePrivateKeyDto key;
        String seed = _seed;
        if (seed != null) {
            key = encryptor.genEncryptKeyNtru(keySize, seed);
        } else {
            key = encryptor.genEncryptKeyNtru(keySize);
        }
        byte[] keyPrivate = key.getPrivateKeyBytes();
        byte[] keyPublic = key.getPublicKeyBytes();
        assert keyPrivate != null : "@AssumeAssertion(nullness): Must not be null";
        assert keyPublic != null : "@AssumeAssertion(nullness): Must not be null";

        String plain = "test";

        byte[] enc = encryptor.encryptNtruWithPublic(keyPublic, plain.getBytes());
        byte[] plainBytes = encryptor.decryptNtruWithPrivate(keyPrivate, enc);

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

    @Test
    public void testFixedEncryptKey() throws IOException, InvalidCipherTextException {
        MessagePrivateKeyDto key = new MessagePrivateKeyDto("hCtNNY27gTrDwo2k1w_nm-28B_0u0Z8_lJYSqdmlRzpxb1Ke194tDZWyNEUR8uchT89qg_R1erx9CAyHFMYgAS2Gs5xfRy_37N2JmtR43HmEVDwcoytHjahdZGNYDIEzrSPhJuAb62unOwNjtS0LF9vkXR5akiyaxz7S21sKCitYwonYjGnODaf4axN6H6n_jhhHIHsGORK_o-Giq7FKZNJhoVfyEaNZPsHkG763cKKSKzkvHHVt7EONjW1OjFT6O5E0gNtiGDKQRquJBtWQUlsosDTaXCQWedj6HzBKsXQZjT_XL5QDSsUHIfTN4oiPqiNHREtjUuWMPa1GsOwhPSDRYpcsscBcD67gKRPeuk4_LfqwPk77ibEdbbP4g1FJhn8eaIGpXWTMFWG5Y_z8PfzS98K46Rj_dkHctVen3lHP_MiitAiUp4FtMdBl_FCHhpKFtoU0mriEUyjm1vLxxmgMuDVxb2Szo3Lm3Rgjq2ZSQBj9Sea-GuqBwc_7uBkqZY-vb72FqQ54jy0-CP73Ij4uJ_uH2g93pJDzSfxPtmsZOp7Rs5pYT03gWr018llG4D4Xtsm-2xP_IONLasoJHTrkkg9XPvmxZSQ8_AUSLZfoGRjWxKrYS1qZqCoZ9zYf_x1UtQEpDFjs__Zo9JONKMieTTskykXv-SwSIiyA6EUbvBTN4-VFVZNmc8zCkBDRRH2jZZUCMbYGkuMXEO_aIM2YwYpRROUj48p7zo8uYlnB82YHvhb6czGWew-RSfNeMeE1vX2Z9qoVQRPgj-5dKbnG2Xbkifmjj4h4Aw", "hCtNNY27gTrDwo2k1w_nm-28B_0u0Z8_lJYSqdmlRzpxb1Ke194tDZWyNEUR8uchT89qg_R1erx9CAyHFMYgAS2Gs5xfRy_37N2JmtR43HmEVDwcoytHjahdZGNYDIEzrSPhJuAb62unOwNjtS0LF9vkXR5akiyaxz7S21sKCitYwonYjGnODaf4axN6H6n_jhhHIHsGORK_o-Giq7FKZNJhoVfyEaNZPsHkG763cKKSKzkvHHVt7EONjW1OjFT6O5E0gNtiGDKQRquJBtWQUlsosDTaXCQWedj6HzBKsXQZjT_XL5QDSsUHIfTN4oiPqiNHREtjUuWMPa1GsOwhPSDRYpcsscBcD67gKRPeuk4_LfqwPk77ibEdbbP4g1FJhn8eaIGpXWTMFWG5Y_z8PfzS98K46Rj_dkHctVen3lHP_MiitAiUp4FtMdBl_FCHhpKFtoU0mriEUyjm1vLxxmgMuDVxb2Szo3Lm3Rgjq2ZSQBj9Sea-GuqBwc_7uBkqZY-vb72FqQ54jy0-CP73Ij4uJ_uH2g93pJDzSfxPtmsZOp7Rs5pYT03gWr018llG4D4Xtsm-2xP_IONLasoJHTrkkg9XPvmxZSQ8_AUSLZfoGRjWxKrYS1qZqCoZ9zYf_x1UtQEpDFjs__Zo9JONKMieTTskykXv-SwSIiyA6EUbvBTN4-VFVZNmc8zCkBDRRH2jZZUCMbYGkuMXEO_aIM2YwYpRROUj48p7zo8uYlnB82YHvhb6czGWew-RSfNeMeE1vX2Z9qoVQRPgj-5dKbnG2Xbkifmjj4h4A35nyKJ3ikeM8yUi_FlKfk_c3f8Tacpp7F8UZUunoUF2VDvYohoTyU6FrHBK-PqRIKU-4HBkrR2LF6Y2zyABrr3C5axkSVArak7ofFERtX0shq9aj4OmCg");
        key.setAlias("public");

        byte[] keyPrivate = key.getPrivateKeyBytes();
        byte[] keyPublic = key.getPublicKeyBytes();
        assert keyPrivate != null : "@AssumeAssertion(nullness): Must not be null";
        assert keyPublic != null : "@AssumeAssertion(nullness): Must not be null";

        String plain = "test";

        byte[] enc = encryptor.encryptNtruWithPublic(keyPublic, plain.getBytes());
        byte[] plainBytes = encryptor.decryptNtruWithPrivate(keyPrivate, enc);

        String test = new String(plainBytes);
        Assertions.assertEquals(plain, test);
    }

    @Test
    public void testFixedSignKey() {
        MessagePrivateKeyDto key = new MessagePrivateKeyDto("rz39v_ev9aFHHJrhE0bn7RONg_RqfGNDXpARYuja8yHO2vf4npuodKpgMApzJW73V0-giMMXyweuYTP3fDtrrdQ_p-3hhAK91wqharZDf18PiU1HOzjFCAWSyQF6eDMzpAwoSUk1_sfL2nUTqF5s_oMlPkHcClBABvm0S3fKvJQC-HLPDpFFaCnsfStu-8ytyx_gjPnBSuGnL1qz5w", "AM232z_XLRsxcxJsNsjcDHJtj-Su62y7jTTn_QE4eFAA6ctcftImbHfTm04nfAmf5EhYcadcPzuwIdRZagyBOADleiEpAXtf4YqQnDX42scZvELRLoEjpofzo2Q5ncLKAOLkz9iZc3oS6PQpS8AZbEcrVq8qhSh_8MjpwYdDpG6vPf2_96_1oUccmuETRuftE42D9Gp8Y0NekBFi6NrzIc7a9_iem6h0qmAwCnMlbvdXT6CIwxfLB65hM_d8O2ut1D-n7eGEAr3XCqFqtkN_Xw-JTUc7OMUIBZLJAXp4MzOkDChJSTX-x8vadROoXmz-gyU-QdwKUEAG-bRLd8q8lAL4cs8OkUVoKex9K277zK3LH-CM-cFK4acvWrPnrz39v_ev9aFHHJrhE0bn7RONg_RqfGNDXpARYuja8yHO2vf4npuodKpgMApzJW73V0-giMMXyweuYTP3fDtrrdQ_p-3hhAK91wqharZDf18PiU1HOzjFCAWSyQF6eDMzpAwoSUk1_sfL2nUTqF5s_oMlPkHcClBABvm0S3fKvJQC-HLPDpFFaCnsfStu-8ytyx_gjPnBSuGnL1qz5w");
        key.setAlias("public");

        byte[] keyPrivate = key.getPrivateKeyBytes();
        byte[] keyPublic = key.getPublicKeyBytes();
        assert keyPrivate != null : "@AssumeAssertion(nullness): Must not be null";
        assert keyPublic != null : "@AssumeAssertion(nullness): Must not be null";

        String plain = "test";
        byte[] digest = encryptor.hashSha(null, plain.getBytes());

        byte[] sig = encryptor.signNtru(keyPrivate, digest);
        boolean isValid = encryptor.verifyNtru(keyPublic, digest, sig);
        Assertions.assertTrue(isValid);
    }

    //@Test
    public void generateSignKeys() {
        for (int n = 0; n < 4; n++) {
            MessagePrivateKeyDto key = encryptor.genSignKeyNtru(64);
            //System.out.println(yamlDelegate.serializeObj(key));
        }
    }

    //@Test
    public void generateEncryptKeys() {
        for (int n = 0; n < 32; n++) {
            MessagePrivateKeyDto key = encryptor.genEncryptKeyNtru(128);
            //System.out.println(yamlDelegate.serializeObj(key));
        }
    }
}
