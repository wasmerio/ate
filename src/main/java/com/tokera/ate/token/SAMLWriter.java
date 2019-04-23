package com.tokera.ate.token;

import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.filters.DefaultBootstrapInit;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.joda.time.DateTime;
import org.opensaml.Configuration;
import org.opensaml.common.SAMLObjectBuilder;
import org.opensaml.common.SAMLVersion;
import org.opensaml.saml2.core.Assertion;
import org.opensaml.saml2.core.Attribute;
import org.opensaml.saml2.core.AttributeStatement;
import org.opensaml.saml2.core.AttributeValue;
import org.opensaml.saml2.core.AuthnContext;
import org.opensaml.saml2.core.AuthnContextClassRef;
import org.opensaml.saml2.core.AuthnStatement;
import org.opensaml.saml2.core.Condition;
import org.opensaml.saml2.core.Conditions;
import org.opensaml.saml2.core.Issuer;
import org.opensaml.saml2.core.NameID;
import org.opensaml.saml2.core.OneTimeUse;
import org.opensaml.saml2.core.Subject;
import org.opensaml.saml2.core.SubjectConfirmation;
import org.opensaml.saml2.core.SubjectConfirmationData;
import org.opensaml.saml2.core.impl.AssertionMarshaller;
import org.opensaml.xml.ConfigurationException;
import org.opensaml.xml.XMLObjectBuilder;
import org.opensaml.xml.XMLObjectBuilderFactory;
import org.opensaml.xml.io.MarshallingException;
import org.opensaml.xml.schema.XSString;
import org.opensaml.xml.security.SecurityException;
import org.opensaml.xml.signature.SignatureException;
import org.opensaml.xml.util.XMLHelper;
import org.w3c.dom.Element;

import java.util.*;
import javax.enterprise.inject.spi.CDI;
import javax.ws.rs.WebApplicationException;

/**
 * This class is used to creates a valid SAML 2.0 Assertion.
 */
public class SAMLWriter {

	@Nullable
	private static XMLObjectBuilderFactory builderFactory;

	public static TokenDto createToken(
            String companyName, String reference, String id, String nameQualifier, Map<String, List<String>> claims, int expiresMins
	) {
            try {
                SAMLInputContainer input = new SAMLInputContainer();
                input.strIssuer = "http://api." + companyName;
                input.strRecipient = "http://api." + companyName + "/*";
                input.strReference = reference;
                input.strNameID = id;
                input.strNameQualifier = nameQualifier;
                input.sessionId = UUID.randomUUID().toString();
                input.attributes = claims;

                if (expiresMins > 0) {
                    input.maxSessionTimeoutInMinutes = expiresMins;
                }

                // Create the assertion and sign it
                Assertion assertion = SAMLWriter.buildDefaultAssertion(input);
                //assertion.setSignature();

                // Sign the assertion
                SignAssertion signer = new SignAssertion();
                signer.signAssertion(assertion);

                // Convert the assertion to a string
                AssertionMarshaller marshaller = new AssertionMarshaller();
                Element plaintextElement = marshaller.marshall(assertion);
                String originalAssertionString = XMLHelper.nodeToString(plaintextElement);

                // System.out.println("Assertion String: " + originalAssertionString);
                // TODO: now you can also add encryption....
                String signedAssertionString = originalAssertionString;

                // Return the token to the caller
                return new TokenDto(signedAssertionString);

            } catch (MarshallingException | SignatureException | SecurityException ex)
            {
                throw new WebApplicationException("Failed to generate token, rReference:" + reference, ex, javax.ws.rs.core.Response.Status.INTERNAL_SERVER_ERROR);
            }
	}

	public static XMLObjectBuilderFactory getSAMLBuilder() throws ConfigurationException {
		XMLObjectBuilderFactory builderFactory = SAMLWriter.builderFactory;
		if (builderFactory == null) {
			// OpenSAML 2.3
			CDI.current().select(DefaultBootstrapInit.class).get().touch();
			builderFactory = Configuration.getBuilderFactory();
			SAMLWriter.builderFactory = builderFactory;
		}

		return builderFactory;
	}

	/**
	 * Builds a SAML Attribute of type String
	 *
	 * @param name
	 * @param value
	 * @param builderFactory
	 * @return
	 * @throws ConfigurationException
	 */
	public static Attribute buildStringAttribute(
		String name, String value, XMLObjectBuilderFactory builderFactory
	) throws ConfigurationException {
		SAMLObjectBuilder attrBuilder = (SAMLObjectBuilder) getSAMLBuilder().getBuilder(Attribute.DEFAULT_ELEMENT_NAME);
		Attribute attrFirstName = (Attribute) attrBuilder.buildObject();
		attrFirstName.setName(name);

		// Set custom Attributes
		XMLObjectBuilder stringBuilder = getSAMLBuilder().getBuilder(XSString.TYPE_NAME);
		XSString attrValueFirstName = (XSString) stringBuilder.buildObject(AttributeValue.DEFAULT_ELEMENT_NAME, XSString.TYPE_NAME);
		attrValueFirstName.setValue(value);

		attrFirstName.getAttributeValues().add(attrValueFirstName);
		return attrFirstName;
	}

	/**
	 * Builds a SAML Attribute of type String
	 *
	 * @param name
	 * @param values
	 * @param builderFactory
	 * @return
	 * @throws ConfigurationException
	 */
	public static Attribute buildStringAttribute(
		String name, Collection<String> values, XMLObjectBuilderFactory builderFactory
	) throws ConfigurationException {
		SAMLObjectBuilder attrBuilder = (SAMLObjectBuilder) getSAMLBuilder().getBuilder(Attribute.DEFAULT_ELEMENT_NAME);
		Attribute attr = (Attribute) attrBuilder.buildObject();
		attr.setName(name);

		// Set custom Attributes
		XMLObjectBuilder stringBuilder = getSAMLBuilder().getBuilder(XSString.TYPE_NAME);
		for (String value : values) {
			XSString attrValue = (XSString) stringBuilder.buildObject(AttributeValue.DEFAULT_ELEMENT_NAME, XSString.TYPE_NAME);
			attrValue.setValue(value);
			attr.getAttributeValues().add(attrValue);
		}

		return attr;
	}

	/**
	 * Helper method which includes some basic SAML fields which are part of
	 * almost every SAML Assertion.
	 * @param input
	 * @return
	 */
	public static Assertion buildDefaultAssertion(SAMLInputContainer input) {
		try {
			// Calculate when the assertion was calculated and when it expires
			DateTime now = new DateTime();
			DateTime expires = now.plusMinutes(input.getMaxSessionTimeoutInMinutes());


			// Create the NameIdentifier
			SAMLObjectBuilder nameIdBuilder = (SAMLObjectBuilder) SAMLWriter.getSAMLBuilder().getBuilder(NameID.DEFAULT_ELEMENT_NAME);
			NameID nameId = (NameID) nameIdBuilder.buildObject();
			nameId.setFormat(NameID.ENTITY);

            String strNameId = input.getStrNameID();
            if (strNameId != null) {
                nameId.setValue(strNameId);
            }

            String strNameQualifier = input.getStrNameQualifier();
            if (strNameQualifier != null) {
                nameId.setNameQualifier(strNameQualifier);
            }

			// Create the SubjectConfirmation
			SAMLObjectBuilder confirmationMethodBuilder = (SAMLObjectBuilder) SAMLWriter.getSAMLBuilder().getBuilder(SubjectConfirmationData.DEFAULT_ELEMENT_NAME);
			SubjectConfirmationData confirmationMethod = (SubjectConfirmationData) confirmationMethodBuilder.buildObject();

			String strReference = input.strReference;
			if (strReference != null) {
				confirmationMethod.setInResponseTo(strReference);
			}

			String strRecipient = input.strRecipient;
			if (strRecipient != null) {
				confirmationMethod.setRecipient(strRecipient);
			}

			confirmationMethod.setNotBefore(now);
			confirmationMethod.setNotOnOrAfter(expires);

			SAMLObjectBuilder subjectConfirmationBuilder = (SAMLObjectBuilder) SAMLWriter.getSAMLBuilder().getBuilder(SubjectConfirmation.DEFAULT_ELEMENT_NAME);
			SubjectConfirmation subjectConfirmation = (SubjectConfirmation) subjectConfirmationBuilder.buildObject();
			subjectConfirmation.setSubjectConfirmationData(confirmationMethod);

			// Create the Subject
			SAMLObjectBuilder subjectBuilder = (SAMLObjectBuilder) SAMLWriter.getSAMLBuilder().getBuilder(Subject.DEFAULT_ELEMENT_NAME);
			Subject subject = (Subject) subjectBuilder.buildObject();

			subject.setNameID(nameId);
			subject.getSubjectConfirmations().add(subjectConfirmation);

			// Create Authentication Statement
			SAMLObjectBuilder authStatementBuilder = (SAMLObjectBuilder) SAMLWriter.getSAMLBuilder().getBuilder(AuthnStatement.DEFAULT_ELEMENT_NAME);
			AuthnStatement authnStatement = (AuthnStatement) authStatementBuilder.buildObject();
            //authnStatement.setSubject(subject);
			//authnStatement.setAuthenticationMethod(strAuthMethod);
			authnStatement.setAuthnInstant(now);
			authnStatement.setSessionNotOnOrAfter(expires);

			String sessionId = input.getSessionId();
			if (sessionId != null) {
                authnStatement.setSessionIndex(sessionId);
            }

			SAMLObjectBuilder authContextBuilder = (SAMLObjectBuilder) SAMLWriter.getSAMLBuilder().getBuilder(AuthnContext.DEFAULT_ELEMENT_NAME);
			AuthnContext authnContext = (AuthnContext) authContextBuilder.buildObject();

			SAMLObjectBuilder authContextClassRefBuilder = (SAMLObjectBuilder) SAMLWriter.getSAMLBuilder().getBuilder(AuthnContextClassRef.DEFAULT_ELEMENT_NAME);
			AuthnContextClassRef authnContextClassRef = (AuthnContextClassRef) authContextClassRefBuilder.buildObject();
			authnContextClassRef.setAuthnContextClassRef("urn:oasis:names:tc:SAML:2.0:ac:classes:Password"); // TODO not sure exactly about this

			authnContext.setAuthnContextClassRef(authnContextClassRef);
			authnStatement.setAuthnContext(authnContext);

			// Builder Attributes
			SAMLObjectBuilder attrStatementBuilder = (SAMLObjectBuilder) SAMLWriter.getSAMLBuilder().getBuilder(AttributeStatement.DEFAULT_ELEMENT_NAME);
			AttributeStatement attrStatement = (AttributeStatement) attrStatementBuilder.buildObject();

			// Create the attribute statement
			Map<String, List<String>> attributes = input.getAttributes();
			if (attributes != null) {
                Set<String> keySet = attributes.keySet();
                for (String key : keySet) {
                    if (attributes.get(key) == null
                            || attributes.get(key).size() <= 0) {
                        continue;
                    }

                    Attribute attrClaim = buildStringAttribute(key, attributes.get(key), getSAMLBuilder());
                    attrStatement.getAttributes().add(attrClaim);
                }
            }

			SAMLObjectBuilder conditionsBuilder = (SAMLObjectBuilder) SAMLWriter.getSAMLBuilder().getBuilder(Conditions.DEFAULT_ELEMENT_NAME);
			Conditions conditions = (Conditions) conditionsBuilder.buildObject();

			// Create the do-not-cache condition
			SAMLObjectBuilder doNotCacheConditionBuilder = (SAMLObjectBuilder) SAMLWriter.getSAMLBuilder().getBuilder(OneTimeUse.DEFAULT_ELEMENT_NAME);
			Condition condition = (Condition) doNotCacheConditionBuilder.buildObject();
			conditions.getConditions().add(condition);

			// Create the time window conditions
			conditions.setNotBefore(now);
			conditions.setNotOnOrAfter(expires);

			// Create Issuer
			SAMLObjectBuilder issuerBuilder = (SAMLObjectBuilder) SAMLWriter.getSAMLBuilder().getBuilder(Issuer.DEFAULT_ELEMENT_NAME);
			Issuer issuer = (Issuer) issuerBuilder.buildObject();

			String strIssuer = input.getStrIssuer();
			if (strIssuer != null) {
                issuer.setValue(strIssuer);
            }

			// Create the assertion
			SAMLObjectBuilder assertionBuilder = (SAMLObjectBuilder) SAMLWriter.getSAMLBuilder().getBuilder(Assertion.DEFAULT_ELEMENT_NAME);
			Assertion assertion = (Assertion) assertionBuilder.buildObject();
			assertion.setSubject(subject);
			assertion.setIssuer(issuer);
			assertion.setIssueInstant(now);
			assertion.setVersion(SAMLVersion.VERSION_20);

			assertion.getAuthnStatements().add(authnStatement);
			assertion.getAttributeStatements().add(attrStatement);
			assertion.setConditions(conditions);

			return assertion;
		} catch (ConfigurationException ex) {
			throw new WebApplicationException("Unexpected exception while building default assertion", ex, javax.ws.rs.core.Response.Status.INTERNAL_SERVER_ERROR);
		}
	}

	public static class SAMLInputContainer {

		@Nullable
		private String strIssuer;
		@Nullable
		private String strNameID;
		@Nullable
		private String strNameQualifier;
		@Nullable
		private String sessionId;
		@Nullable
		private String strReference;
		@Nullable
		private String strRecipient;
		private int maxSessionTimeoutInMinutes = 15; // default is 15 minutes

		private Map<String, List<String>> attributes = new HashMap<>();

		/**
		 * Returns the strIssuer.
		 *
		 * @return the strIssuer
		 */
		public @Nullable String getStrIssuer() {
			return strIssuer;
		}

		/**
		 * Sets the strIssuer.
		 *
		 * @param strIssuer the strIssuer to set
		 */
		public void setStrIssuer(String strIssuer) {
			this.strIssuer = strIssuer;
		}

		/**
		 * Returns the strNameID.
		 *
		 * @return the strNameID
		 */
		public @Nullable String getStrNameID() {
			return strNameID;
		}

		/**
		 * Sets the strNameID.
		 *
		 * @param strNameID the strNameID to set
		 */
		public void setStrNameID(String strNameID) {
			this.strNameID = strNameID;
		}

		/**
		 * Returns the strNameQualifier.
		 *
		 * @return the strNameQualifier
		 */
		public @Nullable String getStrNameQualifier() {
			return strNameQualifier;
		}

		/**
		 * Sets the strNameQualifier.
		 *
		 * @param strNameQualifier the strNameQualifier to set
		 */
		public void setStrNameQualifier(String strNameQualifier) {
			this.strNameQualifier = strNameQualifier;
		}

		/**
		 * Sets the attributes.
		 *
		 * @param attributes the attributes to set
		 */
		public void setAttributes(Map<String, List<String>> attributes) {
			this.attributes = attributes;
		}

		/**
		 * Returns the attributes.
		 *
		 * @return the attributes
		 */
		public @Nullable Map<String, List<String>> getAttributes() {
			return attributes;
		}

		/**
		 * Sets the sessionId.
		 *
		 * @param sessionId the sessionId to set
		 */
		public void setSessionId(String sessionId) {
			this.sessionId = sessionId;
		}

		/**
		 * Returns the sessionId.
		 *
		 * @return the sessionId
		 */
		public @Nullable String getSessionId() {
			return sessionId;
		}

		/**
		 * Sets the maxSessionTimeoutInMinutes.
		 *
		 * @param maxSessionTimeoutInMinutes the maxSessionTimeoutInMinutes to
		 * set
		 */
		public void setMaxSessionTimeoutInMinutes(int maxSessionTimeoutInMinutes) {
			this.maxSessionTimeoutInMinutes = maxSessionTimeoutInMinutes;
		}

		/**
		 * Returns the maxSessionTimeoutInMinutes.
		 *
		 * @return the maxSessionTimeoutInMinutes
		 */
		public int getMaxSessionTimeoutInMinutes() {
			return maxSessionTimeoutInMinutes;
		}
	}
}
