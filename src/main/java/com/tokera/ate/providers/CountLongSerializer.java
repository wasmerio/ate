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
import com.tokera.ate.dao.CountLong;
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
public class CountLongSerializer extends Serializer<CountLong> implements ScalarSerializer<CountLong>, MessageBodyReader<CountLong>, MessageBodyWriter<CountLong> {
    public CountLongSerializer() {
    }

    @Override
    public void write(Kryo kryo, Output output, @Nullable CountLong pid) {
        String val = CountLong.serialize(pid);
        output.writeString(val);
    }

    @Override
    public @Nullable CountLong read(Kryo kryo, Input input, Class<? extends CountLong> aClass) {
        return CountLong.parse(input.readString());
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable String write(@Nullable CountLong t) {
        return CountLong.serialize(t);
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable CountLong read(@Nullable String val) {
        return CountLong.parse(val);
    }

    @Override
    public boolean isReadable(Class<?> aClass, Type type, Annotation[] annotations, MediaType mediaType) {
        return CountLong.class.isAssignableFrom(aClass);
    }

    @SuppressWarnings("return.type.incompatible")
    @Override
    public @Nullable CountLong readFrom(Class<CountLong> aClass, Type type, Annotation[] annotations, MediaType mediaType, MultivaluedMap<String, String> multivaluedMap, InputStream inputStream) throws IOException, WebApplicationException {
        String txt = IOUtils.toString(inputStream, com.google.common.base.Charsets.UTF_8);
        return CountLong.parse(txt);
    }

    @Override
    public boolean isWriteable(Class<?> aClass, Type type, Annotation[] annotations, MediaType mediaType) {
        return CountLong.class.isAssignableFrom(aClass);
    }

    @Override
    public void writeTo(@Nullable CountLong pid, Class<?> aClass, Type type, Annotation[] annotations, MediaType mediaType, MultivaluedMap<String, Object> multivaluedMap, OutputStream outputStream) throws IOException, WebApplicationException {
        OutputStreamWriter streamWriter = new OutputStreamWriter(outputStream);
        streamWriter.write(CountLong.serialize(pid));
    }
}
