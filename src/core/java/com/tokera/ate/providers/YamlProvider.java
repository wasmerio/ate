package com.tokera.ate.providers;

import com.esotericsoftware.yamlbeans2.YamlException;
import com.esotericsoftware.yamlbeans2.YamlReader;
import com.esotericsoftware.yamlbeans2.YamlWriter;
import com.google.common.base.Charsets;
import com.google.common.io.CharStreams;
import com.tokera.ate.delegates.AteDelegate;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.ws.rs.Consumes;
import javax.ws.rs.Produces;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.MediaType;
import javax.ws.rs.core.MultivaluedMap;
import javax.ws.rs.core.StreamingOutput;
import javax.ws.rs.ext.MessageBodyReader;
import javax.ws.rs.ext.MessageBodyWriter;
import javax.ws.rs.ext.Provider;
import java.io.*;
import java.lang.annotation.Annotation;
import java.lang.reflect.Type;
import java.util.List;
import java.util.Map;
import java.util.Set;

/**
 * Serialization provider for resteasy that adds YAML serialization support
 */
@Provider
@Consumes({"text/yaml", "text/x-yaml", "application/x-yaml"})
@Produces({"text/yaml", "text/x-yaml", "application/x-yaml"})
public class YamlProvider implements MessageBodyWriter<Object>, MessageBodyReader<Object>
{
    protected AteDelegate d = AteDelegate.get();
    
    public boolean isReadable(Class<?> type, Type genericType, Annotation[] annotations, MediaType mediaType)
    {
        return true;
    }

    @Override
    @SuppressWarnings( "deprecation" )
    public Object readFrom(Class<Object> type, Type genericType, Annotation[] annotations, MediaType mediaType,
                           MultivaluedMap<String, String> httpHeaders, InputStream entityStream) throws IOException {
        String yaml = CharStreams.toString(new InputStreamReader(entityStream, Charsets.UTF_8));
        YamlReader reader = d.yaml.getYamlReader(yaml);
        return reader.read();
    }

    protected boolean isValidType(Class type)
    {
        if (List.class.isAssignableFrom(type)
                || Set.class.isAssignableFrom(type)
                || Map.class.isAssignableFrom(type)
                || type.isArray())
        {
            return true;
        }
        if (StreamingOutput.class.isAssignableFrom(type)) return false;
        String className = type.getName();
        if (className.startsWith("java.")) return false;
        if (className.startsWith("javax.")) return false;
        if (type.isPrimitive()) return false;

        return true;
    }

    @Override
    public boolean isWriteable(Class<?> type, Type genericType, Annotation[] annotations, MediaType mediaType)
    {
        return isValidType(type);
    }

    @Override
    public void writeTo(Object t, Class<?> type, Type genericType, Annotation[] annotations, MediaType mediaType,
                        MultivaluedMap<String, Object> httpHeaders, OutputStream entityStream) throws IOException {
        StringWriter sb = new StringWriter();
        YamlWriter writer = d.yaml.getYamlWriter(sb, true);
        writer.write(t);
        writer.close();

        entityStream.write(sb.toString().getBytes());
    }

    @Override
    public long getSize(Object o, Class<?> type, Type genericType, Annotation[] annotations, MediaType mediaType) {
        try {
            return dump(o).getBytes().length;
        } catch (YamlException e) {
            throw new WebApplicationException(e);
        }
    }

    public static String dump(Object t) throws YamlException {
        StringWriter sb = new StringWriter();
        YamlWriter writer = AteDelegate.get().yaml.getYamlWriter(sb, true);
        writer.write(t);
        writer.close();
        return sb.toString();
    }
}