/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.common;

import com.tokera.ate.dao.ILogable;
import com.tokera.ate.delegates.LoggingDelegate;

import java.lang.reflect.Member;
import java.util.Date;

import com.tokera.ate.delegates.RequestContextDelegate;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.slf4j.Marker;

import javax.annotation.PostConstruct;
import javax.enterprise.context.Dependent;
import javax.enterprise.context.RequestScoped;
import javax.enterprise.context.spi.Context;
import javax.enterprise.inject.spi.BeanManager;
import javax.enterprise.inject.spi.InjectionPoint;
import javax.inject.Inject;

/**
 * Custom logger that will direct log commands either to the currentRights logger or to the static logger depending on
 * the scope and settings of the running application.
 */
@Dependent
public class LoggerHook implements org.slf4j.Logger {

    private Class<?> logClazz;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private InjectionPoint injectionPoint;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    public BeanManager beanManager;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    public LoggingDelegate loggingDelegate;

    private static boolean forceStatic = true;
    private static @Nullable Boolean forceContextLogger = null;
    
    public LoggerHook() {
        logClazz = LoggerHook.class;
    }
    
    public LoggerHook(Class<?> clazz) {
        logClazz = clazz;
    }

    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible"})
    public @Nullable Member getInjectionMemberOrNull() {
        return this.injectionPoint.getMember();
    }

    @PostConstruct
    public void init() {
        Member member = getInjectionMemberOrNull();
        if (member != null) {
            logClazz = member.getDeclaringClass();
        }
    }

    public LoggerHook setLogClazz(Class<?> clazz) {
        this.logClazz = clazz;
        return this;
    }
    
    public void pushContext(ILogable context)
    {
        if (RequestContextDelegate.isWithinRequestContext() == false) return;
        loggingDelegate.getLogStack().push(context);
        
        context.setError(null);
        context.setLog(null);
    }
    
    public void popContext() {
        if (RequestContextDelegate.isWithinRequestContext() == false) return;
        loggingDelegate.getLogStack().pop();
    }
    
    protected org.slf4j.Logger getStaticForwader() {
        return org.slf4j.LoggerFactory.getLogger(logClazz);
    }

    protected org.slf4j.Logger getForwarder() {
        
        if (RequestContextDelegate.isWithinRequestContext() == true &&
            LoggerHook.getForceStatic() == false)
        {
            Boolean forceContextLogger = LoggerHook.getForceContextLogger();
            if (forceContextLogger == null) {
                if (loggingDelegate != null && loggingDelegate.getLogStack() != null && loggingDelegate.getLogStack().empty() == false) {
                    return new LoggerToRequest(getStaticForwader(), loggingDelegate);
                } else {
                    return getStaticForwader();
                }
            } else {
                if (forceContextLogger == true && loggingDelegate != null) {
                    return new LoggerToRequest(getStaticForwader(), loggingDelegate);
                } else {
                    return getStaticForwader();
                }
            }
        } else {
            return getStaticForwader();
        }
    }

    public boolean getIsStatic() {
        if (RequestContextDelegate.isWithinRequestContext() == true &&
            LoggerHook.getForceStatic() == false)
        {
            Boolean forceContextLogger = LoggerHook.getForceContextLogger();
            if (forceContextLogger == null) {
                if (loggingDelegate.getLogStack().empty() == false) {
                    return false;
                } else {
                    return true;
                }
            } else {
                if (forceContextLogger == true) {
                    return false;
                } else {
                    return true;
                }
            }
        } else {
            return true;
        }
    }
    
    public void prefixIndent() {
        if (RequestContextDelegate.isWithinRequestContext() == false) return;
        loggingDelegate.setLogPrefix(loggingDelegate.getLogPrefix() + "...");
    }
    
    public void prefixDeindent() {
        if (RequestContextDelegate.isWithinRequestContext() == false) return;
        if (loggingDelegate.getLogPrefix().length() > 3) {
            loggingDelegate.setLogPrefix(loggingDelegate.getLogPrefix().substring(0, loggingDelegate.getLogPrefix().length() - 3));
        } else {
            loggingDelegate.setLogPrefix("");
        }
    }

    @Override
    public String getName() {
        return this.getForwarder().getName();
    }

    @Override
    public boolean isTraceEnabled() {
        return this.getForwarder().isTraceEnabled();
    }

    @Override
    public void trace(String string) {
        this.getForwarder().trace(string);
    }

    @Override
    public void trace(String string, Object o) {
        this.getForwarder().trace(string, o);
    }

    @Override
    public void trace(String string, Object o, Object o1) {
        this.getForwarder().trace(string, o, o1);
    }

    @Override
    public void trace(String string, Object[] os) {
        this.getForwarder().trace(string, os);
    }

    @Override
    public void trace(String string, Throwable thrwbl) {
        this.getForwarder().trace(string, thrwbl);
    }

    @Override
    public boolean isTraceEnabled(Marker marker) {
        return this.getForwarder().isTraceEnabled(marker);
    }

    @Override
    public void trace(Marker marker, String string) {
        this.getForwarder().trace(marker, string);
    }

    @Override
    public void trace(Marker marker, String string, Object o) {
        this.getForwarder().trace(marker, string, o);
    }

    @Override
    public void trace(Marker marker, String string, Object o, Object o1) {
        this.getForwarder().trace(marker, string, o, o1);
    }

    @Override
    public void trace(Marker marker, String string, Object[] os) {
        this.getForwarder().trace(marker, string, os);
    }

    @Override
    public void trace(Marker marker, String string, Throwable thrwbl) {
        this.getForwarder().trace(marker, string, thrwbl);
    }

    @Override
    public boolean isDebugEnabled() {
        return this.getForwarder().isDebugEnabled();
    }

    @Override
    public void debug(String message) {
        this.getForwarder().debug(message);
    }

    @Override
    public void debug(String string, Object o) {
        this.getForwarder().debug(string, o);
    }

    @Override
    public void debug(String string, Object o, Object o1) {
        this.getForwarder().debug(string, o, o1);
    }

    @Override
    public void debug(String string, Object[] os) {
        this.getForwarder().debug(string, os);
    }

    @Override
    public void debug(String string, Throwable thrwbl) {
        this.getForwarder().debug(string, thrwbl);
    }

    @Override
    public boolean isDebugEnabled(Marker marker) {
        return this.getForwarder().isDebugEnabled(marker);
    }

    @Override
    public void debug(Marker marker, String string) {
        this.getForwarder().debug(marker, string);
    }

    @Override
    public void debug(Marker marker, String string, Object o) {
        this.getForwarder().debug(marker, string, o);
    }

    @Override
    public void debug(Marker marker, String string, Object o, Object o1) {
        this.getForwarder().debug(marker, string, o, o1);
    }

    @Override
    public void debug(Marker marker, String string, Object[] os) {
        this.getForwarder().debug(marker, string, os);
    }

    @Override
    public void debug(Marker marker, String string, Throwable thrwbl) {
        this.getForwarder().debug(marker, string, thrwbl);
    }

    @Override
    public boolean isInfoEnabled() {
        return this.getForwarder().isInfoEnabled();
    }

    @Override
    public void info(String string) {
        this.getForwarder().info(string);
    }

    @Override
    public void info(String string, Object o) {
        this.getForwarder().info(string, o);
    }

    @Override
    public void info(String string, Object o, Object o1) {
        this.getForwarder().info(string, o, o1);
    }

    @Override
    public void info(String string, Object[] os) {
        this.getForwarder().info(string, os);
    }

    @Override
    public void info(String string, Throwable thrwbl) {
        this.getForwarder().info(string, thrwbl);
    }

    @Override
    public boolean isInfoEnabled(Marker marker) {
        return this.getForwarder().isInfoEnabled(marker);
    }

    @Override
    public void info(Marker marker, String string) {
        this.getForwarder().info(marker, string);
    }

    @Override
    public void info(Marker marker, String string, Object o) {
        this.getForwarder().info(marker, string, o);
    }

    @Override
    public void info(Marker marker, String string, Object o, Object o1) {
        this.getForwarder().info(marker, string, o, o1);
    }

    @Override
    public void info(Marker marker, String string, Object[] os) {
        this.getForwarder().info(marker, string, os);
    }

    @Override
    public void info(Marker marker, String string, Throwable thrwbl) {
        this.getForwarder().info(marker, string, thrwbl);
    }
    
    public void infoWithTime(String msg)
    {
        this.info(msg + "(at " + (new Date()) + ")");
    }

    @Override
    public boolean isWarnEnabled() {
        return this.getForwarder().isWarnEnabled();
    }

    @Override
    public void warn(String string) {
        this.getForwarder().warn(string);
    }

    @Override
    public void warn(String string, Object o) {
        this.getForwarder().warn(string, o);
    }

    @Override
    public void warn(String string, Object[] os) {
        this.getForwarder().warn(string, os);
    }

    @Override
    public void warn(String string, Object o, Object o1) {
        this.getForwarder().warn(string, o, o1);
    }

    @Override
    public void warn(String string, Throwable thrwbl) {
        this.getForwarder().warn(string, thrwbl);
    }

    public void warn(Throwable thrwbl) {
        String msg = thrwbl.getMessage();
        if (msg == null) msg = thrwbl.toString();
        this.getForwarder().warn(msg, thrwbl);
    }

    @Override
    public boolean isWarnEnabled(Marker marker) {
        return this.getForwarder().isWarnEnabled(marker);
    }

    @Override
    public void warn(Marker marker, String string) {
        this.getForwarder().warn(marker, string);
    }

    @Override
    public void warn(Marker marker, String string, Object o) {
        this.getForwarder().warn(marker, string, o);
    }

    @Override
    public void warn(Marker marker, String string, Object o, Object o1) {
        this.getForwarder().warn(marker, string, o, o1);
    }

    @Override
    public void warn(Marker marker, String string, Object[] os) {
        this.getForwarder().warn(marker, string, os);
    }

    @Override
    public void warn(Marker marker, String string, Throwable thrwbl) {
        this.getForwarder().warn(marker, string, thrwbl);
    }

    @Override
    public boolean isErrorEnabled() {
        return this.getForwarder().isErrorEnabled();
    }

    @Override
    public void error(String string) {
        this.getForwarder().error(string);
    }

    @Override
    public void error(String string, Object o) {
        this.getForwarder().error(string, o);
    }

    @Override
    public void error(String string, Object o, Object o1) {
        this.getForwarder().error(string, o, o1);
    }

    @Override
    public void error(String string, Object[] os) {
        this.getForwarder().error(string, os);
    }

    @Override
    public void error(String string, Throwable thrwbl) {
        this.getForwarder().error(string, thrwbl);
    }

    public void error(Throwable thrwbl) {
        String msg = thrwbl.getMessage();
        if (msg == null) msg = thrwbl.toString();
        this.getForwarder().error(msg, thrwbl);
    }

    @Override
    public boolean isErrorEnabled(Marker marker) {
        return this.getForwarder().isErrorEnabled(marker);
    }

    @Override
    public void error(Marker marker, String string) {
        this.getForwarder().error(marker, string);
    }

    @Override
    public void error(Marker marker, String string, Object o) {
        this.getForwarder().error(marker, string, o);
    }

    @Override
    public void error(Marker marker, String string, Object o, Object o1) {
        this.getForwarder().error(marker, string, o, o1);
    }

    @Override
    public void error(Marker marker, String string, Object[] os) {
        this.getForwarder().error(marker, string, os);
    }

    @Override
    public void error(Marker marker, String string, Throwable thrwbl) {
        this.getForwarder().error(marker, string, thrwbl);
    }

    public static boolean getForceStatic() {
        return LoggerHook.forceStatic;
    }

    public static void setForceStatic(boolean forceStatic) {
        LoggerHook.forceStatic = forceStatic;
    }

    public static @Nullable Boolean getForceContextLogger() {
        return LoggerHook.forceContextLogger;
    }

    public static void setForceContextLogger(boolean forceContextLogger) {
        LoggerHook.forceContextLogger = forceContextLogger;
    }
}
