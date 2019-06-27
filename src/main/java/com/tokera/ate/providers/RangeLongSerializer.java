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
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.RangeLong;
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
public class RangeLongSerializer extends Serializer<RangeLong> implements ScalarSerializer<RangeLong>, MessageBodyReader<RangeLong>, MessageBodyWriter<RangeLong>
{
    public RangeLongSerializer() {
        
    }

    @Override
    public void write(Kryo kryo, Output output, RangeLong object) {
        String val = this.write(object);
        output.writeString(val);
    }

    @Override
    public RangeLong read(Kryo kryo, Input input, Class<? extends RangeLong> type) {
        return read(input.readString());
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable String write(@Nullable RangeLong t) {
        if (t == null) return "null";
        return t.getMinimumLong() + ":" + t.getMaximumLong();
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable RangeLong read(@Nullable String _val) {
        String val = StringTools.makeOneLineOrNull(_val);
        val = StringTools.specialParse(val);
        if (val == null || val.length() <= 0) return null;

        String[] comps = val.split(":");
        if (comps.length != 2) return null;

        Long min = Long.parseLong(comps[0]);
        Long max = Long.parseLong(comps[1]);
        return new RangeLong(min, max);
    }

    @Override
    public boolean isReadable(Class<?> type, Type genericType, Annotation[] annotations, MediaType mediaType) {
        return RangeLong.class.isAssignableFrom(type);
    }

    @Override
    public RangeLong readFrom(Class<RangeLong> type, Type genericType, Annotation[] annotations, MediaType mediaType, MultivaluedMap<String, String> httpHeaders, InputStream entityStream) throws IOException, WebApplicationException {
        String txt = IOUtils.toString(entityStream, com.google.common.base.Charsets.UTF_8);
        return this.read(txt);
    }

    @Override
    public boolean isWriteable(Class<?> type, Type genericType, Annotation[] annotations, MediaType mediaType) {
        return RangeLong.class.isAssignableFrom(type);
    }

    @Override
    public void writeTo(RangeLong rangeLong, Class<?> type, Type genericType, Annotation[] annotations, MediaType mediaType, MultivaluedMap<String, Object> httpHeaders, OutputStream entityStream) throws IOException, WebApplicationException {
        String txt = this.write(rangeLong);
        if (txt == null) txt = "null";
        OutputStreamWriter streamWriter = new OutputStreamWriter(entityStream);
        streamWriter.write(txt);
    }
}
