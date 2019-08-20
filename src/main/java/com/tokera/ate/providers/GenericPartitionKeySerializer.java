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
import com.tokera.ate.dao.GenericPartitionKey;
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
public class GenericPartitionKeySerializer extends Serializer<GenericPartitionKey> implements ScalarSerializer<GenericPartitionKey>, MessageBodyReader<GenericPartitionKey>, MessageBodyWriter<GenericPartitionKey> {
    public GenericPartitionKeySerializer() {
    }

    @Override
    public void write(Kryo kryo, Output output, GenericPartitionKey partitionKey) {
        String val = this.write(partitionKey);
        output.writeString(val);
    }

    @Override
    public @Nullable GenericPartitionKey read(Kryo kryo, Input input, Class<? extends GenericPartitionKey> aClass) {
        return this.read(input.readString());
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable String write(@Nullable GenericPartitionKey t) {
        if (t == null) return "null";
        return this.toString(t);
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable GenericPartitionKey read(@Nullable String val) {
        return parse(val);
    }

    @Override
    public boolean isReadable(Class<?> aClass, Type type, Annotation[] annotations, MediaType mediaType) {
        return GenericPartitionKey.class.isAssignableFrom(aClass);
    }

    @SuppressWarnings("return.type.incompatible")
    @Override
    public GenericPartitionKey readFrom(Class<GenericPartitionKey> aClass, Type type, Annotation[] annotations, MediaType mediaType, MultivaluedMap<String, String> multivaluedMap, InputStream inputStream) throws IOException, WebApplicationException {
        String txt = IOUtils.toString(inputStream, com.google.common.base.Charsets.UTF_8);
        return this.read(txt);
    }

    @Override
    public boolean isWriteable(Class<?> aClass, Type type, Annotation[] annotations, MediaType mediaType) {
        return GenericPartitionKey.class.isAssignableFrom(aClass);
    }

    @Override
    public void writeTo(GenericPartitionKey key, Class<?> aClass, Type type, Annotation[] annotations, MediaType mediaType, MultivaluedMap<String, Object> multivaluedMap, OutputStream outputStream) throws IOException, WebApplicationException {
        String txt = this.write(key);
        if (txt == null) txt = "null";
        OutputStreamWriter streamWriter = new OutputStreamWriter(outputStream);
        streamWriter.write(txt);
    }

    public static String toString(GenericPartitionKey key) {
        return PartitionKeySerializer.toString(key);
    }

    public static String serialize(GenericPartitionKey key) {
        return PartitionKeySerializer.toString(key);
    }

    public static int hashCode(GenericPartitionKey key) {
        return PartitionKeySerializer.hashCode(key);
    }

    public static boolean equals(@Nullable GenericPartitionKey left, @Nullable Object rightObj) {
        return PartitionKeySerializer.equals(left, rightObj);
    }

    public static int compareTo(@Nullable GenericPartitionKey left, @Nullable GenericPartitionKey right) {
        return PartitionKeySerializer.compareTo(left, right);
    }

    public static @Nullable GenericPartitionKey parse(@Nullable String val) {
        IPartitionKey ret = PartitionKeySerializer.parse(val);
        return new GenericPartitionKey(ret);
    }
}
