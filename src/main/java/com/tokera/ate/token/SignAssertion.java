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

    @MonotonicNonNull
    private static BasicX509Credential signingCredential;
    public static final String STORE_PASSWORD = "7E264A281750DBEA5F15269D47AF1003877426D5EF7F99C4E739E0C9942C58470F15E678C32FB99B";
    public static final String STS_PASSWORD = "F4257978B79904B78903AB62C3B9F7EBFF42FDC8ED1F66995584DCD4D9E27E1082563FE92D7078A4";
    public static final String CERTIFICATE_ALIAS_NAME = "sts";
    public static final String FILENAME = "/token.signing.jks";

    private static BasicX509Credential intializeCredentials() {
        BasicX509Credential cred;
        if (signingCredential == null) {
            CDI.current().select(DefaultBootstrapInit.class).get().touch();
            cred = getSigningCredential();
            signingCredential = cred;
        } else {
            cred = signingCredential;
        }
        return cred;
    }

    public static BasicX509Credential getSigningCredentialCached() {
        BasicX509Credential ret = signingCredential;
        if (ret != null) {
            return ret;
        }
        return intializeCredentials();
    }

    public static BasicX509Credential getSigningCredential() {
        try {
            try (InputStream fis = SignAssertion.class.getResourceAsStream(FILENAME)) {
                char[] store_password = SignAssertion.STORE_PASSWORD.toCharArray();

                // Get Default Instance of KeyStore
                KeyStore ks = KeyStore.getInstance(KeyStore.getDefaultType());
                if (fis == null) {
                    throw new WebApplicationException("Failed to open signing certificate [" + FILENAME + "]", javax.ws.rs.core.Response.Status.INTERNAL_SERVER_ERROR);
                }
                ks.load(fis, store_password);

                // Get Private Key Entry From Certificate
                KeyStore.PrivateKeyEntry pkEntry = (KeyStore.PrivateKeyEntry) ks.getEntry(SignAssertion.CERTIFICATE_ALIAS_NAME,
                        new KeyStore.PasswordProtection(SignAssertion.STS_PASSWORD.toCharArray()));
                PrivateKey pk = pkEntry.getPrivateKey();

                X509Certificate certificate = (X509Certificate) pkEntry.getCertificate();
                BasicX509Credential credential = new BasicX509Credential();
                credential.setEntityCertificate(certificate);
                credential.setPrivateKey(pk);

                //this.LOG.log(Level.INFO, "Private Key{0}", pk.toString());
                return credential;
            }
        } catch (IOException | KeyStoreException | NoSuchAlgorithmException | CertificateException | UnrecoverableEntryException ex) {
            throw new WebApplicationException("Exception occured when signing credentials.", ex, javax.ws.rs.core.Response.Status.INTERNAL_SERVER_ERROR);
        }
    }

    public void signAssertion(Assertion assertion) throws MarshallingException, SignatureException, SecurityException {
        // Create the class that will perform the signing
        BasicX509Credential creds = SignAssertion.intializeCredentials();

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
