package com.tokera.ate.providers;

import java.io.IOException;
import java.io.InputStream;
import java.lang.annotation.Annotation;
import java.lang.reflect.Type;

import javax.ws.rs.core.MediaType;
import javax.ws.rs.core.MultivaluedMap;

import com.google.common.io.ByteStreams;

import javax.ws.rs.Consumes;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.ext.MessageBodyReader;
import javax.ws.rs.ext.Provider;

/**
 * Serializer for resteasy that reads a byte stream and copies it to an InputStream
 */
@Provider
@Consumes("application/octet-stream")
public class ProcessBodyReader implements MessageBodyReader<byte[]> {

    @Override
    public boolean isReadable(Class<?> t, Type gt, Annotation[] as, MediaType mediaType) {
        return byte[].class.isAssignableFrom(t);
    }

    @Override
    public byte[] readFrom(Class<byte[]> type, Type type1, Annotation[] antns, MediaType mt, MultivaluedMap<String, String> mm, InputStream in) throws IOException, WebApplicationException {
        return ByteStreams.toByteArray(in);
    }
}
