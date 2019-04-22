package com.tokera.ate.providers;

import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.lang.annotation.Annotation;
import java.lang.reflect.Type;
import javax.ws.rs.Produces;

import javax.ws.rs.core.MediaType;
import javax.ws.rs.core.MultivaluedMap;
import javax.ws.rs.ext.MessageBodyWriter;
import javax.ws.rs.ext.Provider;
import org.apache.commons.io.IOUtils;

/**
 * Serializer for rest easy that copies InputStreams into the REST return byte stream
 */
@Provider
@Produces("application/octet-stream")
public class ProcessBodyWriter implements MessageBodyWriter<InputStream> {

    @Override
    public boolean isWriteable(Class<?> t, Type gt, Annotation[] as, MediaType mediaType) {
        return InputStream.class.isAssignableFrom(t);
    }

    @Override
    public long getSize(InputStream o, Class<?> type, Type genericType, Annotation[] annotations, MediaType mediaType) {
        return -1;
    }

    @Override
    public void writeTo(InputStream o, Class<?> t, Type gt, Annotation[] as,
            MediaType mediaType, MultivaluedMap<String, Object> httpHeaders,
            OutputStream entity) throws IOException
    {
        IOUtils.copy(o, entity);
    }
}
