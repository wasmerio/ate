package com.tokera.ate.token;

import java.io.IOException;
import java.io.InputStream;
import java.security.KeyStore;
import java.security.KeyStoreException;
import java.security.NoSuchAlgorithmException;
import java.security.PrivateKey;
import java.security.UnrecoverableEntryException;
import java.security.cert.CertificateException;
import java.security.cert.X509Certificate;
import javax.enterprise.inject.spi.CDI;
import javax.ws.rs.WebApplicationException;

import com.tokera.ate.common.ApplicationConfigLoader;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.filters.DefaultBootstrapInit;
import org.checkerframework.checker.nullness.qual.MonotonicNonNull;
import org.opensaml.Configuration;
import org.opensaml.saml2.core.Assertion;
import org.opensaml.xml.io.MarshallingException;
import org.opensaml.xml.security.SecurityConfiguration;
import org.opensaml.xml.security.SecurityException;
import org.opensaml.xml.security.SecurityHelper;
import org.opensaml.xml.security.x509.BasicX509Credential;
import org.opensaml.xml.signature.Signature;
import org.opensaml.xml.signature.SignatureConstants;
import org.opensaml.xml.signature.SignatureException;
import org.opensaml.xml.signature.Signer;

public class SignAssertion {

    private static @MonotonicNonNull BasicX509Credential signingCredential;

    public static BasicX509Credential getSigningCredential() {
        BasicX509Credential ret = SignAssertion.signingCredential;
        if (ret != null) {
            return ret;
        }
        CDI.current().select(DefaultBootstrapInit.class).get().touch();
        ret =  createSigningCredential();
        SignAssertion.signingCredential = ret;
        return ret;
    }

    public static BasicX509Credential createSigningCredential() {
        try {
            AteDelegate d = AteDelegate.get();
            String where = d.bootstrapConfig.getStsVaultFilename();
            try (InputStream fis = ApplicationConfigLoader.getInstance().getResourceByName(where)) {
                char[] store_password = d.bootstrapConfig.getStsVaultPassword().toCharArray();

                // Get Default Instance of KeyStore
                KeyStore ks = KeyStore.getInstance(KeyStore.getDefaultType());
                if (fis == null) {
                    throw new WebApplicationException("Failed to open signing certificate [" + d.bootstrapConfig.getStsVaultFilename() + "]", javax.ws.rs.core.Response.Status.INTERNAL_SERVER_ERROR);
                }
                ks.load(fis, store_password);

                // Get Private Key Entry From Certificate
                KeyStore.PrivateKeyEntry pkEntry = (KeyStore.PrivateKeyEntry) ks.getEntry(d.bootstrapConfig.getStsCertificateAliasName(),
                        new KeyStore.PasswordProtection(d.bootstrapConfig.getStsSigningKeyPassword().toCharArray()));
                PrivateKey pk = pkEntry.getPrivateKey();

                X509Certificate certificate = (X509Certificate) pkEntry.getCertificate();
                BasicX509Credential credential = new BasicX509Credential();
                credential.setEntityCertificate(certificate);
                credential.setPrivateKey(pk);
                return credential;
            }
        } catch (IOException | KeyStoreException | NoSuchAlgorithmException | CertificateException | UnrecoverableEntryException ex) {
            throw new WebApplicationException("Exception occured when signing credentials.", ex, javax.ws.rs.core.Response.Status.INTERNAL_SERVER_ERROR);
        }
    }

    public void signAssertion(Assertion assertion) throws MarshallingException, SignatureException, SecurityException {
        // Create the class that will perform the signing
        BasicX509Credential creds = SignAssertion.getSigningCredential();

        // Get the signature object and set it up
        Signature signature = (Signature) Configuration.getBuilderFactory()
                .getBuilder(Signature.DEFAULT_ELEMENT_NAME).buildObject(Signature.DEFAULT_ELEMENT_NAME);

        // Set the signing params
        SecurityConfiguration secConfig = Configuration.getGlobalSecurityConfiguration();
        SecurityHelper.prepareSignatureParams(signature, creds, secConfig, "");

        signature.setSigningCredential(creds);
        signature.setSignatureAlgorithm(SignatureConstants.ALGO_ID_SIGNATURE_RSA_SHA512);
        signature.setCanonicalizationAlgorithm(SignatureConstants.ALGO_ID_C14N_EXCL_OMIT_COMMENTS);

        // Set the signature
        assertion.setSignature(signature);

        // Marshall and sign the assertion
        Configuration.getMarshallerFactory().getMarshaller(assertion).marshall(assertion);
        Signer.signObject(signature);
    }
}
