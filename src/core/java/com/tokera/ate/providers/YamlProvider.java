package com.tokera.ate.providers;

import com.esotericsoftware.yamlbeans2.YamlConfig;
import com.esotericsoftware.yamlbeans2.YamlException;
import com.esotericsoftware.yamlbeans2.YamlReader;
import com.esotericsoftware.yamlbeans2.YamlWriter;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.extensions.YamlTagDiscoveryExtension;
import com.tokera.ate.delegates.YamlDelegate;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.jboss.resteasy.logging.Logger;
import org.jboss.resteasy.spi.ReaderException;
import org.jboss.resteasy.spi.WriterException;

import javax.ws.rs.Consumes;
import javax.ws.rs.Produces;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.MediaType;
import javax.ws.rs.core.MultivaluedMap;
import javax.ws.rs.core.StreamingOutput;
import javax.ws.rs.ext.Provider;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.io.StringWriter;
import java.lang.annotation.Annotation;
import java.lang.reflect.Type;
import java.util.List;
import java.util.Map;
import java.util.Set;

import org.jboss.resteasy.plugins.providers.AbstractEntityProvider;

/**
 * Serialization provider for resteasy that adds YAML serialization support
 */
@Provider
@Consumes({"text/yaml", "text/x-yaml", "application/x-yaml"})
@Produces({"text/yaml", "text/x-yaml", "application/x-yaml"})
public class YamlProvider extends AbstractEntityProvider<Object>
{
    final static Logger logger = Logger.getLogger(YamlProvider.class);

    protected AteDelegate d = AteDelegate.get();
    
    public boolean isReadable(Class<?> type, Type genericType, Annotation[] annotations, MediaType mediaType)
    {
        return true;
    }

    @SuppressWarnings( "deprecation" )
    public Object readFrom(Class<Object> type, Type genericType, Annotation[] annotations, MediaType mediaType,
                           MultivaluedMap<String, String> httpHeaders, InputStream entityStream) throws IOException,
            WebApplicationException
    {
        try
        {
            String yaml = org.apache.commons.io.IOUtils.toString(entityStream);
            YamlReader reader = d.yaml.getYamlReader(yaml);
            return reader.read();
        }
        catch (YamlException ye)
        {
            String msg = ye.getMessage();
            if (msg == null) msg = ye.getClass().getSimpleName();
            logger.debug("Failed to decode Yaml: {0}", msg);
            throw new ReaderException("Failed to decode Yaml", ye);
        }
        catch (Exception e)
        {
            String msg = e.getMessage();
            if (msg == null) msg = e.getClass().getSimpleName();
            logger.debug("Failed to decode Yaml: {0}", msg);
            throw new ReaderException("Failed to decode Yaml", e);
        }
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


    public boolean isWriteable(Class<?> type, Type genericType, Annotation[] annotations, MediaType mediaType)
    {
        return isValidType(type);
    }

    public void writeTo(Object t, Class<?> type, Type genericType, Annotation[] annotations, MediaType mediaType,
                        MultivaluedMap<String, Object> httpHeaders, OutputStream entityStream) throws IOException,
            WebApplicationException
    {
        try
        {
            StringWriter sb = new StringWriter();
            YamlWriter writer = d.yaml.getYamlWriter(sb, true);
            writer.write(t);
            writer.close();
        
            entityStream.write(sb.toString().getBytes());
        }
        catch (Exception e)
        {
            logger.debug("Failed to encode yaml for object: {0}", t.toString());
            throw new WriterException(e);
        }
    }
    
    public static @Nullable String dump(Object t)
    {
        try
        {
            YamlTagDiscoveryExtension discovery = javax.enterprise.inject.spi.CDI.current().select(YamlTagDiscoveryExtension.class).get();
            
            StringWriter sb = new StringWriter();
            YamlWriter writer = new YamlWriter(sb);

            YamlConfig config = writer.getConfig();
            if (config == null) throw new WebApplicationException("Missing configuration object in YamlWriter");

            YamlDelegate.initConfig(config, discovery);
            writer.write(t);
            writer.close();
            
            return sb.toString();
        }
        catch (YamlException ex)
        {
            logger.debug("Failed to encode yaml for object: {0}", t.toString());
            return null;
        }
    }
}