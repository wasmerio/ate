/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.providers;

import com.esotericsoftware.kryo.Kryo;
import com.esotericsoftware.kryo.Serializer;
import com.esotericsoftware.kryo.io.Input;
import com.esotericsoftware.kryo.io.Output;
import com.esotericsoftware.yamlbeans.scalar.ScalarSerializer;
import com.fasterxml.jackson.core.JsonGenerator;
import com.fasterxml.jackson.core.JsonParser;
import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.DeserializationContext;
import com.fasterxml.jackson.databind.SerializerProvider;
import com.fasterxml.jackson.databind.deser.std.StdScalarDeserializer;
import com.fasterxml.jackson.databind.ser.std.StdScalarSerializer;
import com.tokera.ate.common.StringTools;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.io.api.IPartitionKey;
import org.apache.commons.io.IOUtils;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.ws.rs.Consumes;
import javax.ws.rs.Produces;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.MediaType;
import javax.ws.rs.core.MultivaluedMap;
import javax.ws.rs.ext.MessageBodyReader;
import javax.ws.rs.ext.MessageBodyWriter;
import javax.ws.rs.ext.Provider;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.io.OutputStreamWriter;
import java.lang.annotation.Annotation;
import java.lang.reflect.Type;

@Provider
@Consumes("text/plain")
@Produces("text/plain")
public class PuuidSerializer extends Serializer<PUUID> implements ScalarSerializer<PUUID>, MessageBodyReader<PUUID>, MessageBodyWriter<PUUID> {
    public PuuidSerializer() {
    }

    @Override
    public void write(Kryo kryo, Output output, @Nullable PUUID pid) {
        String val = PUUID.serialize(pid);
        output.writeString(val);
    }

    @Override
    public @Nullable PUUID read(Kryo kryo, Input input, Class<? extends PUUID> aClass) {
        return PUUID.parse(input.readString());
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable String write(@Nullable PUUID t) {
        return PUUID.serialize(t);
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable PUUID read(@Nullable String val) {
        return PUUID.parse(val);
    }

    @Override
    public boolean isReadable(Class<?> aClass, Type type, Annotation[] annotations, MediaType mediaType) {
        if (aClass == PUUID.class) return true;
        return PUUID.class.isAssignableFrom(aClass);
    }

    @SuppressWarnings("return.type.incompatible")
    @Override
    public @Nullable PUUID readFrom(Class<PUUID> aClass, Type type, Annotation[] annotations, MediaType mediaType, MultivaluedMap<String, String> multivaluedMap, InputStream inputStream) throws IOException, WebApplicationException {
        String txt = IOUtils.toString(inputStream, com.google.common.base.Charsets.UTF_8);
        return PUUID.parse(txt);
    }

    @Override
    public boolean isWriteable(Class<?> aClass, Type type, Annotation[] annotations, MediaType mediaType) {
        if (aClass == PUUID.class) return true;
        return PUUID.class.isAssignableFrom(aClass);
    }

    @Override
    public void writeTo(@Nullable PUUID pid, Class<?> aClass, Type type, Annotation[] annotations, MediaType mediaType, MultivaluedMap<String, Object> multivaluedMap, OutputStream outputStream) throws IOException, WebApplicationException {
        OutputStreamWriter streamWriter = new OutputStreamWriter(outputStream);
        streamWriter.write(PUUID.serialize(pid));
    }
}
