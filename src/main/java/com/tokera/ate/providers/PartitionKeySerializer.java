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
import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.core.JsonGenerator;
import com.fasterxml.jackson.core.JsonParser;
import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.DeserializationContext;
import com.fasterxml.jackson.databind.SerializerProvider;
import com.fasterxml.jackson.databind.deser.std.StdScalarDeserializer;
import com.fasterxml.jackson.databind.ser.std.StdScalarSerializer;
import com.tokera.ate.common.StringTools;
import com.tokera.ate.common.UUIDTools;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.enumerations.DataPartitionType;
import com.tokera.ate.io.api.IPartitionKey;
import org.apache.commons.codec.binary.Base64;
import org.apache.commons.io.IOUtils;
import org.apache.commons.io.output.ByteArrayOutputStream;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.ws.rs.Consumes;
import javax.ws.rs.Produces;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.MediaType;
import javax.ws.rs.core.MultivaluedMap;
import javax.ws.rs.ext.MessageBodyReader;
import javax.ws.rs.ext.MessageBodyWriter;
import javax.ws.rs.ext.Provider;
import java.io.*;
import java.lang.annotation.Annotation;
import java.lang.reflect.Type;
import java.nio.ByteBuffer;
import java.util.UUID;

@Provider
@Consumes("text/plain")
@Produces("text/plain")
public class PartitionKeySerializer extends Serializer<IPartitionKey> implements ScalarSerializer<IPartitionKey>, MessageBodyReader<IPartitionKey>, MessageBodyWriter<IPartitionKey> {
    public PartitionKeySerializer() {
    }

    public static class PartitionKeyValue implements IPartitionKey {
        private final String m_partitionTopic;
        private final int m_partitionIndex;
        private final DataPartitionType m_partitionType;
        @JsonIgnore
        private transient String m_base64;

        public PartitionKeyValue(String topic, int index, DataPartitionType type) {
            this.m_partitionTopic = topic;
            this.m_partitionIndex = index;
            this.m_partitionType = type;
        }

        @Override
        public String partitionTopic() {
            return m_partitionTopic;
        }

        @Override
        public int partitionIndex() {
            return m_partitionIndex;
        }

        @Override
        public DataPartitionType partitionType() { return m_partitionType; }

        @Override
        public String asBase64() {
            if (m_base64 != null) return m_base64;
            m_base64 = PartitionKeySerializer.serialize(this);
            return m_base64;
        }

        @Override
        public String toString() {
            return PartitionKeySerializer.toString(this);
        }

        @Override
        public int hashCode() {
            return PartitionKeySerializer.hashCode(this);
        }

        @Override
        public boolean equals(Object val) {
            return PartitionKeySerializer.equals(this, val);
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
        return t.asBase64();
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable IPartitionKey read(@Nullable String val) {
        return parse(val);
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

    public static String toString(IPartitionKey key) {
        return key.partitionType().name().toLowerCase() + ":" + key.partitionTopic() + ":" + key.partitionIndex();
    }

    public static String serialize(IPartitionKey key) {
        try {
            ByteArrayOutputStream stream = new ByteArrayOutputStream();
            DataOutputStream dos = new DataOutputStream(stream);
            dos.writeShort(key.partitionType().getCode());
            String topic = key.partitionTopic();
            if (topic != null) {
                dos.writeShort(topic.length());
                dos.write(topic.getBytes());
            } else {
                dos.writeShort(0);
            }
            dos.writeInt(key.partitionIndex());
            return Base64.encodeBase64URLSafeString(stream.toByteArray());
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
    }

    public static int hashCode(IPartitionKey key) {
        return key.asBase64().hashCode();
    }

    public static boolean equals(@Nullable IPartitionKey left, @Nullable Object rightObj) {
        if (left == null && rightObj == null) return true;
        if (left == null) return false;
        if (rightObj == null) return false;
        if (rightObj instanceof IPartitionKey) {
            IPartitionKey right = (IPartitionKey)rightObj;
            String leftVal = left.asBase64();
            String rightVal = right.asBase64();
            return leftVal.equals(rightVal);
        } else {
            return false;
        }
    }

    public static int compareTo(@Nullable IPartitionKey left, @Nullable IPartitionKey right) {
        if (left == null && right == null) return -1;
        if (left == null) return -1;
        if (right == null) return 1;
        String leftVal = left.asBase64();
        String rightVal = right.asBase64();
        return leftVal.compareTo(rightVal);
    }

    public static @Nullable IPartitionKey parse(@Nullable String _val) {
        String val = StringTools.makeOneLineOrNull(_val);
        val = StringTools.specialParse(val);
        if (val == null || val.length() <= 0) return null;

        if (val.contains(":")) {
            String[] comps = val.split(":");
            if (comps.length != 3) return null;

            String type = comps[0];
            String topic = comps[1];
            Integer index = Integer.parseInt(comps[2]);

            return new PartitionKeyValue(
                    topic,
                    index,
                    DataPartitionType.parse(type));
        }

        byte[] data = Base64.decodeBase64(val);
        ByteBuffer bb = ByteBuffer.wrap(data);

        int typeCode = bb.getShort();
        DataPartitionType type = DataPartitionType.fromCode(typeCode);

        String topic = null;
        int topicLen = bb.getShort();
        if (topicLen > 0) {
            byte[] topicBytes = new byte[topicLen];
            bb.get(topicBytes);
            topic = new String(topicBytes);
        }

        int index = bb.getInt();

        return new PartitionKeyValue(
                topic,
                index,
                type);
    }
}
