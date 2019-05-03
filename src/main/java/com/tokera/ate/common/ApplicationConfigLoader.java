package com.tokera.ate.common;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.io.InputStream;
import java.security.SecureRandom;
import java.util.Properties;

import org.checkerframework.checker.nullness.qual.Nullable;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import javax.ws.rs.WebApplicationException;


/**
 * Class used to parse application configuration files and use these to override default settings
 */
public class ApplicationConfigLoader {
    
    private static ApplicationConfigLoader g_Singleton;
    private static final Logger LOG = LoggerFactory.getLogger(ApplicationConfigLoader.class);
    
    private long currentVersion = 0;

    static {
        g_Singleton = new ApplicationConfigLoader();
    }
    
    public ApplicationConfigLoader() {
    }
    
    public static ApplicationConfigLoader getInstance() {
        return g_Singleton;
    }
    
    public static long getCurrentVersion() {
        if (g_Singleton.currentVersion != 0) return g_Singleton.currentVersion;        
        Properties props = g_Singleton.getPropertiesByName("version.properties");
        if (props == null) return 0L;
        
        if ("<<RANDOM-VERSION>>".equals(props.getProperty("version")) == true ||
            "<<TOKAPI-VERSION>>".equals(props.getProperty("version")) == true) {
            SecureRandom srandom = new SecureRandom();
            g_Singleton.currentVersion = srandom.nextLong();
        } else {
            String propName = props.getProperty("version");
            if (propName == null) return 0L;
            g_Singleton.currentVersion = Long.parseLong(propName);
            if (g_Singleton.currentVersion == 0L) g_Singleton.currentVersion = 1L;
        }
        return g_Singleton.currentVersion;
    }
    
    public @Nullable Properties getPropertiesByName(@Nullable String _name) {

        String name = _name;
        if (name == null) {
            return null;
        }
        
        Properties versionProps = new Properties();
        try {
            FileInputStream input = null;
            input = new FileInputStream(new File(name));
            versionProps.load(input);
        } catch (FileNotFoundException e) {
            InputStream input = null;
            try {
                ClassLoader loader = this.getClass().getClassLoader();
                if (loader == null) {
                    throw new WebApplicationException("No class loader found for this class.");
                }
                input = loader.getResourceAsStream(name);
                if (input == null) {
                    throw new FileNotFoundException("Could not find resource stream [" + name + "]");
                }
                versionProps.load(input);
            } catch (FileNotFoundException ex) {
                String msg = ex.getMessage();
                if (msg == null) msg = ex.getClass().getSimpleName();
                LOG.warn(msg, ex);
                return null;
            } catch (IOException ex) {
                String msg = ex.getMessage();
                if (msg == null) msg = ex.getClass().getSimpleName();
                LOG.warn(msg, ex);
                return null;
            } finally {
                try {
                    if (input != null) {
                        input.close();
                    }
                } catch (IOException ex) {
                    String msg = ex.getMessage();
                    if (msg == null) msg = ex.getClass().getSimpleName();
                    LOG.warn(msg, ex);
                }
            }
        } catch (IOException ex) {
            String msg = ex.getMessage();
            if (msg == null) msg = ex.getClass().getSimpleName();
            LOG.warn(msg, ex);
            return null;
        }
        return versionProps;
    }
}
