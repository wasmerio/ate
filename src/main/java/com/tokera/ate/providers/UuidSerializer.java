/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.providers;

import com.esotericsoftware.yamlbeans.YamlException;
import com.esotericsoftware.yamlbeans.scalar.ScalarSerializer;
import com.tokera.ate.common.StringTools;
import com.tokera.ate.common.UUIDTools;
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
public class UuidSerializer implements ScalarSerializer<UUID>, MessageBodyReader<UUID>, MessageBodyWriter<UUID>
{
    public UuidSerializer() {
        
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable String write(@Nullable UUID t) throws YamlException {
        if (t == null) return "null";
        return t.toString();
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable UUID read(@Nullable String _val) throws YamlException {
        String val = StringTools.makeOneLineOrNull(_val);
        val = StringTools.specialParse(val);
        if (val == null || val.length() <= 0) return null;

        return UUIDTools.parseUUIDorNull(val);
    }

    @Override
    public boolean isReadable(Class<?> aClass, Type type, Annotation[] annotations, MediaType mediaType) {
        return UUID.class.isAssignableFrom(aClass);
    }

    @Override
    public UUID readFrom(Class<UUID> aClass, Type type, Annotation[] annotations, MediaType mediaType, MultivaluedMap<String, String> multivaluedMap, InputStream inputStream) throws IOException, WebApplicationException {
        String txt = IOUtils.toString(inputStream, com.google.common.base.Charsets.UTF_8);
        return this.read(txt);
    }

    @Override
    public boolean isWriteable(Class<?> aClass, Type type, Annotation[] annotations, MediaType mediaType) {
        return UUID.class.isAssignableFrom(aClass);
    }

    @Override
    public void writeTo(UUID uuid, Class<?> aClass, Type type, Annotation[] annotations, MediaType mediaType, MultivaluedMap<String, Object> multivaluedMap, OutputStream outputStream) throws IOException, WebApplicationException {
        String txt = this.write(uuid);
        OutputStreamWriter streamWriter = new OutputStreamWriter(outputStream);
        streamWriter.write(txt);
    }
}
