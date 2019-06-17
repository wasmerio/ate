package com.tokera.ate.common;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.io.InputStream;
import java.security.SecureRandom;
import java.util.Collection;
import java.util.Properties;

import com.tokera.ate.delegates.AteDelegate;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import javax.ws.rs.WebApplicationException;
import java.util.ArrayList;
import java.util.Enumeration;
import java.util.regex.Pattern;
import java.util.zip.ZipEntry;
import java.util.zip.ZipException;
import java.util.zip.ZipFile;

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
        if (name == null) return null;

        InputStream input = getResourceByName(name);
        try {
            Properties versionProps = new Properties();
            versionProps.load(input);
            return versionProps;
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
    }

    public @Nullable InputStream getResourceByName(@Nullable String _name) {
        String name = _name;
        if (name == null) return null;

        ClassLoader loader = AteDelegate.get().bootstrapConfig.getApplicationClass().getClassLoader();
        InputStream ret = getResourceByNameInternal(loader, name);
        if (ret != null) return ret;

        loader = this.getClass().getClassLoader();
        ret = getResourceByNameInternal(loader, name);
        if (ret != null) return ret;

        return getResourceByName(this.getClass().getClassLoader(), name, true);
    }

    private @Nullable InputStream getResourceByNameInternal(ClassLoader loader, String name) {
        InputStream ret = getResourceByName(loader, name, false);
        if (ret != null) return ret;
        if (name.startsWith("/") == false) {
            ret = getResourceByName(loader, "/" + name, false);
            if (ret != null) return ret;
        }
        return null;
    }
    
    public @Nullable InputStream getResourceByName(ClassLoader loader, @Nullable String _name, boolean shouldThrow) {
        String name = _name;
        if (name == null) return null;
        
        try {
            return new FileInputStream(new File(name));
        }
        catch (FileNotFoundException e) {
            InputStream input = null;
            try {
                if (loader == null) {
                    if (shouldThrow == false) return null;
                    throw new WebApplicationException("No class loader found for this class.");
                }
                input = loader.getResourceAsStream(name);
                if (input == null) {
                    if (shouldThrow == false) return null;
                    throw new FileNotFoundException("Could not find resource stream [" + name + "]");
                }
                return input;
            } catch (FileNotFoundException ex) {
                String msg = ex.getMessage();
                if (msg == null) msg = ex.getClass().getSimpleName();
                LOG.warn(msg, ex);
                return null;
            }
        }
    }
}
