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
import com.esotericsoftware.yamlbeans.YamlException;
import com.esotericsoftware.yamlbeans.scalar.ScalarSerializer;
import com.tokera.ate.common.StringTools;
import com.tokera.ate.common.UUIDTools;
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
import java.util.UUID;

@Provider
@Consumes("text/plain")
@Produces("text/plain")
public class PartitionKeySerializer extends Serializer<IPartitionKey> implements ScalarSerializer<IPartitionKey>, MessageBodyReader<IPartitionKey>, MessageBodyWriter<IPartitionKey> {
    public PartitionKeySerializer() {
    }

    public class PartitionKeyValue implements IPartitionKey {
        private final String m_partitionTopic;
        private final int m_partitionIndex;

        public PartitionKeyValue(String topic, int index) {
            this.m_partitionTopic = topic;
            this.m_partitionIndex = index;
        }

        @Override
        public String partitionTopic() {
            return m_partitionTopic;
        }

        @Override
        public int partitionIndex() {
            return m_partitionIndex;
        }
    }

    @Override
    public void write(Kryo kryo, Output output, IPartitionKey partitionKey) {
        String val = this.write(partitionKey);
        output.writeString(val);
    }

    @Override
    public @Nullable IPartitionKey read(Kryo kryo, Input input, Class<? extends IPartitionKey> aClass) {
        return this.read(input.readString());
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable String write(@Nullable IPartitionKey t) {
        if (t == null) return "null";
        return t.partitionTopic() + "-" + t.partitionIndex();
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable IPartitionKey read(@Nullable String _val) {
        String val = StringTools.makeOneLineOrNull(_val);
        val = StringTools.specialParse(val);
        if (val == null || val.length() <= 0) return null;

        String[] comps = val.split("-");
        if (comps.length != 2) return null;

        return new PartitionKeyValue(
                comps[0],
                Integer.parseInt(comps[1]));
    }

    @Override
    public boolean isReadable(Class<?> aClass, Type type, Annotation[] annotations, MediaType mediaType) {
        return IPartitionKey.class.isAssignableFrom(aClass);
    }

    @SuppressWarnings("return.type.incompatible")
    @Override
    public IPartitionKey readFrom(Class<IPartitionKey> aClass, Type type, Annotation[] annotations, MediaType mediaType, MultivaluedMap<String, String> multivaluedMap, InputStream inputStream) throws IOException, WebApplicationException {
        String txt = IOUtils.toString(inputStream, com.google.common.base.Charsets.UTF_8);
        return this.read(txt);
    }

    @Override
    public boolean isWriteable(Class<?> aClass, Type type, Annotation[] annotations, MediaType mediaType) {
        return IPartitionKey.class.isAssignableFrom(aClass);
    }

    @Override
    public void writeTo(IPartitionKey key, Class<?> aClass, Type type, Annotation[] annotations, MediaType mediaType, MultivaluedMap<String, Object> multivaluedMap, OutputStream outputStream) throws IOException, WebApplicationException {
        String txt = this.write(key);
        if (txt == null) txt = "null";
        OutputStreamWriter streamWriter = new OutputStreamWriter(outputStream);
        streamWriter.write(txt);
    }
}
