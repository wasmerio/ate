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
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
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
public class PrivateKeyWithSeedSerializer extends Serializer<PrivateKeyWithSeedDto> implements ScalarSerializer<PrivateKeyWithSeedDto>, MessageBodyReader<PrivateKeyWithSeedDto>, MessageBodyWriter<PrivateKeyWithSeedDto> {

    public PrivateKeyWithSeedSerializer() {
    }

    @Override
    public void write(Kryo kryo, Output output, PrivateKeyWithSeedDto object) {
        String val = object.serialize();
        output.writeString(val);
    }

    @Override
    public PrivateKeyWithSeedDto read(Kryo kryo, Input input, Class<? extends PrivateKeyWithSeedDto> type) {
        return PrivateKeyWithSeedDto.deserialize(input.readString());
    }

    @Override
    public String write(PrivateKeyWithSeedDto object) throws YamlException {
        return object.serialize();
    }

    @Override
    public PrivateKeyWithSeedDto read(String value) throws YamlException {
        return PrivateKeyWithSeedDto.deserialize(value);
    }

    @Override
    public boolean isReadable(Class<?> type, Type genericType, Annotation[] annotations, MediaType mediaType) {
        if (type == PrivateKeyWithSeedDto.class) return true;
        return PrivateKeyWithSeedDto.class.isAssignableFrom(type);
    }

    @Override
    public PrivateKeyWithSeedDto readFrom(Class<PrivateKeyWithSeedDto> type, Type genericType, Annotation[] annotations, MediaType mediaType, MultivaluedMap<String, String> httpHeaders, InputStream entityStream) throws IOException, WebApplicationException {
        String txt = IOUtils.toString(entityStream, com.google.common.base.Charsets.UTF_8);
        return PrivateKeyWithSeedDto.deserialize(txt);
    }

    @Override
    public boolean isWriteable(Class<?> type, Type genericType, Annotation[] annotations, MediaType mediaType) {
        if (type == PrivateKeyWithSeedDto.class) return true;
        return PrivateKeyWithSeedDto.class.isAssignableFrom(type);
    }

    @Override
    public void writeTo(PrivateKeyWithSeedDto key, Class<?> type, Type genericType, Annotation[] annotations, MediaType mediaType, MultivaluedMap<String, Object> httpHeaders, OutputStream entityStream) throws IOException, WebApplicationException {
        OutputStreamWriter streamWriter = new OutputStreamWriter(entityStream);
        streamWriter.write(key.serialize());
    }
}
