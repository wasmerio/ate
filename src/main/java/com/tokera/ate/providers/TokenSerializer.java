package com.tokera.ate.providers;

import com.auth0.jwt.JWT;
import com.auth0.jwt.JWTCreator;
import com.auth0.jwt.JWTVerifier;
import com.auth0.jwt.algorithms.Algorithm;
import com.auth0.jwt.exceptions.JWTDecodeException;
import com.auth0.jwt.exceptions.JWTVerificationException;
import com.auth0.jwt.interfaces.Claim;
import com.auth0.jwt.interfaces.DecodedJWT;
import com.esotericsoftware.kryo.Kryo;
import com.esotericsoftware.kryo.Serializer;
import com.esotericsoftware.kryo.io.Input;
import com.esotericsoftware.kryo.io.Output;
import com.esotericsoftware.yamlbeans.YamlException;
import com.esotericsoftware.yamlbeans.scalar.ScalarSerializer;
import com.tokera.ate.common.ImmutalizableArrayList;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.ClaimDto;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.units.Alias;
import org.apache.commons.codec.binary.Base64;
import org.apache.commons.io.IOUtils;
import org.apache.commons.lang3.time.DateUtils;

import javax.ws.rs.Consumes;
import javax.ws.rs.Produces;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.MediaType;
import javax.ws.rs.core.MultivaluedMap;
import javax.ws.rs.core.Response;
import javax.ws.rs.ext.MessageBodyReader;
import javax.ws.rs.ext.MessageBodyWriter;
import javax.ws.rs.ext.Provider;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.io.OutputStreamWriter;
import java.lang.annotation.Annotation;
import java.lang.reflect.Type;
import java.util.Date;
import java.util.List;
import java.util.Map;
import java.util.Properties;

@Provider
@Consumes("text/plain")
@Produces("text/plain")
public class TokenSerializer implements ScalarSerializer<TokenDto>, MessageBodyReader<TokenDto>, MessageBodyWriter<TokenDto> {
    private AteDelegate d = AteDelegate.get();

    private String jwtSecret;
    private byte[] jwtEncrypt;
    private String jwtIssuer;

    public TokenSerializer() {
        Properties props = d.bootstrapConfig.propertiesForToken();
        this.jwtSecret = props.getOrDefault("secret", "anyone").toString();
        this.jwtEncrypt = Base64.decodeBase64(props.getOrDefault("encrypt", "VD5eE_z1crGougAuE-xubgJwACNzN4aF7h5VrltBsYw").toString());
        this.jwtIssuer = props.getOrDefault("issuer", "nobody").toString();
    }

    public TokenDto createToken(Map<@Alias String, List<String>> claims, int expiresMins) {
        Algorithm algorithm;
        if (d.bootstrapConfig.getSecurityLevel().signToken) {
            algorithm = Algorithm.HMAC256(this.jwtSecret);
        }  else {
            algorithm = Algorithm.none();
        }

        JWTCreator.Builder builder = JWT.create()
                .withIssuer(jwtIssuer);

        for (Map.Entry<String, List<String>> claim : claims.entrySet()) {
            builder = builder.withArrayClaim(claim.getKey(), claim.getValue().stream().toArray(String[]::new));
        }
        if (expiresMins > 0) {
            builder = builder.withExpiresAt(DateUtils.addMinutes(new Date(), expiresMins));
        }

        String base64;
        String plain = builder.sign(algorithm);
        if (d.bootstrapConfig.getSecurityLevel().encryptToken) {
            byte[] enc = d.encryptor.encryptAes(jwtEncrypt, plain.getBytes(), true);
            base64 = Base64.encodeBase64URLSafeString(enc);
        } else {
            base64 = plain;
        }

        return new TokenDto(base64);
    }

    public void validateToken(TokenDto token) {
        String plain;
        if (d.bootstrapConfig.getSecurityLevel().encryptToken) {
            String encToken = token.getBase64();
            byte[] bytes = Base64.decodeBase64(encToken);
            bytes = d.encryptor.decryptAes(jwtEncrypt, bytes, false);
            if (bytes == null) {
                throw new WebApplicationException("JWT token failed decrypt", Response.Status.UNAUTHORIZED);
            }
            plain = new String(bytes);
        } else {
            plain = token.getBase64();
        }

        Algorithm algorithm;
        if (d.bootstrapConfig.getSecurityLevel().signToken) {
            algorithm = Algorithm.HMAC256(this.jwtSecret);
        }  else {
            algorithm = Algorithm.none();
        }

        try {
            JWTVerifier verifier = JWT.require(algorithm)
                    .withIssuer(jwtIssuer)
                    .build(); //Reusable verifier instance
            verifier.verify(plain);
        } catch (JWTVerificationException exception){
            throw new WebApplicationException("JWT token failed validation", exception, Response.Status.UNAUTHORIZED);
        }
    }

    public ImmutalizableArrayList<ClaimDto> extractTokenClaims(TokenDto token) {
        String plain;
        if (d.bootstrapConfig.getSecurityLevel().encryptToken) {
            String encToken = token.getBase64();
            byte[] bytes = Base64.decodeBase64(encToken);
            plain = new String(d.encryptor.decryptAes(jwtEncrypt, bytes, false));
            if (plain == null) {
                return new ImmutalizableArrayList<>();
            }
        } else {
            plain = token.getBase64();
        }

        ImmutalizableArrayList<ClaimDto> ret = new ImmutalizableArrayList<>();
        try {
            DecodedJWT jwt = JWT.decode(plain);
            for (Map.Entry<String, Claim> claim : jwt.getClaims().entrySet()) {
                if (claim.getKey().equals("iss")) continue;
                if (claim.getKey().equals("exp")) continue;
                if (claim.getValue().isNull()) {
                    continue;
                }
                List<String> vals = claim.getValue().asList(String.class);
                if (vals == null) {
                    continue;
                }
                for (String val : vals) {
                    ret.add(new ClaimDto(claim.getKey(), val));
                }
            }
        } catch (JWTDecodeException exception){
            throw new WebApplicationException("Failed to decode the JWT token.", exception, Response.Status.UNAUTHORIZED);
        }

        ret.immutalize();
        return ret;
    }

    @Override
    public String write(TokenDto object) throws YamlException {
        return object.getBase64();
    }

    @Override
    public TokenDto read(String value) throws YamlException {
        return new TokenDto(value);
    }

    @Override
    public boolean isReadable(Class<?> type, Type genericType, Annotation[] annotations, MediaType mediaType) {
        return TokenDto.class.isAssignableFrom(type);
    }

    @Override
    public TokenDto readFrom(Class<TokenDto> type, Type genericType, Annotation[] annotations, MediaType mediaType, MultivaluedMap<String, String> httpHeaders, InputStream entityStream) throws IOException, WebApplicationException {
        String txt = IOUtils.toString(entityStream, com.google.common.base.Charsets.UTF_8);
        return new TokenDto(txt);
    }

    @Override
    public boolean isWriteable(Class<?> type, Type genericType, Annotation[] annotations, MediaType mediaType) {
        return TokenDto.class.isAssignableFrom(type);
    }

    @Override
    public void writeTo(TokenDto tokenDto, Class<?> type, Type genericType, Annotation[] annotations, MediaType mediaType, MultivaluedMap<String, Object> httpHeaders, OutputStream entityStream) throws IOException, WebApplicationException {
        OutputStreamWriter streamWriter = new OutputStreamWriter(entityStream);
        streamWriter.write(tokenDto.getBase64());
    }
}
