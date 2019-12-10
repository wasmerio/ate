/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.common;

import com.tokera.ate.dao.ILogable;
import com.tokera.ate.delegates.LoggingDelegate;

import java.io.IOException;
import java.io.OutputStream;
import java.lang.reflect.Member;
import java.util.Date;
import java.util.function.BiConsumer;
import java.util.function.Consumer;

import com.tokera.ate.delegates.RequestContextDelegate;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.junit.jupiter.api.Assertions;
import org.slf4j.Marker;

import javax.annotation.PostConstruct;
import javax.enterprise.context.Dependent;
import javax.enterprise.inject.spi.BeanManager;
import javax.enterprise.inject.spi.InjectionPoint;
import javax.inject.Inject;
import javax.servlet.http.HttpServletResponse;

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

    private static volatile ConcurrentStack<String> flagWarning = null;
    private static volatile ConcurrentStack<String> flagError = null;
    
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

    public void resumeContext(ILogable context)
    {
        if (RequestContextDelegate.isWithinRequestContext() == false) return;
        loggingDelegate.getLogStack().push(context);

    }
    
    public void popContext() {
        if (RequestContextDelegate.isWithinRequestContext() == false) return;
        loggingDelegate.getLogStack().pop();
    }
    
    protected org.slf4j.Logger getStaticForwarder() {
        return org.slf4j.LoggerFactory.getLogger(logClazz);
    }

    protected org.slf4j.Logger getForwarder() {

        org.slf4j.Logger ret;
        if (RequestContextDelegate.isWithinRequestContext() == true &&
            loggingDelegate.getForceStatic() == false)
        {
            Boolean forceContextLogger = loggingDelegate.getForceContextLogger();
            if (forceContextLogger == null) {
                if (loggingDelegate != null && (loggingDelegate.getLogStack().empty() == false || loggingDelegate.getRedirectStream() != null)) {
                    ret = new LoggerToRequest(getStaticForwarder(), loggingDelegate);
                } else {
                    ret = getStaticForwarder();
                }
            } else {
                if (forceContextLogger == true && loggingDelegate != null) {
                    ret = new LoggerToRequest(getStaticForwarder(), loggingDelegate);
                } else {
                    ret = getStaticForwarder();
                }
            }
        } else {
            ret = getStaticForwarder();
        }
        return ret;
    }

    public boolean getIsStatic() {
        if (RequestContextDelegate.isWithinRequestContext() == true &&
            loggingDelegate.getForceStatic() == false)
        {
            Boolean forceContextLogger = loggingDelegate.getForceContextLogger();
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
        loggingDelegate.setLogPrefix(loggingDelegate.getLogPrefix() + "..");
    }
    
    public void prefixDeindent() {
        if (RequestContextDelegate.isWithinRequestContext() == false) return;
        if (loggingDelegate.getLogPrefix().length() > 2) {
            loggingDelegate.setLogPrefix(loggingDelegate.getLogPrefix().substring(0, loggingDelegate.getLogPrefix().length() - 2));
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
        if (flagWarning != null) flagWarning.push(string);
        this.getForwarder().warn(string);
    }

    @Override
    public void warn(String string, Object o) {
        if (flagWarning != null) flagWarning.push(string);
        this.getForwarder().warn(string, o);
    }

    @Override
    public void warn(String string, Object[] os) {
        if (flagWarning != null) flagWarning.push(string);
        this.getForwarder().warn(string, os);
    }

    @Override
    public void warn(String string, Object o, Object o1) {
        if (flagWarning != null) flagWarning.push(string);
        this.getForwarder().warn(string, o, o1);
    }

    @Override
    public void warn(String string, Throwable thrwbl) {
        if (flagWarning != null) flagWarning.push(string);
        this.getForwarder().warn(string, thrwbl);
    }

    public void warn(Throwable thrwbl) {
        String msg = thrwbl.getMessage();
        if (msg == null) msg = thrwbl.toString();
        if (flagWarning != null) flagWarning.push(msg);
        this.getForwarder().warn(msg, thrwbl);
    }

    @Override
    public boolean isWarnEnabled(Marker marker) {
        return this.getForwarder().isWarnEnabled(marker);
    }

    @Override
    public void warn(Marker marker, String string) {
        if (flagWarning != null) flagWarning.push(string);
        this.getForwarder().warn(marker, string);
    }

    @Override
    public void warn(Marker marker, String string, Object o) {
        if (flagWarning != null) flagWarning.push(string);
        this.getForwarder().warn(marker, string, o);
    }

    @Override
    public void warn(Marker marker, String string, Object o, Object o1) {
        if (flagWarning != null) flagWarning.push(string);
        this.getForwarder().warn(marker, string, o, o1);
    }

    @Override
    public void warn(Marker marker, String string, Object[] os) {
        if (flagWarning != null) flagWarning.push(string);
        this.getForwarder().warn(marker, string, os);
    }

    @Override
    public void warn(Marker marker, String string, Throwable thrwbl) {
        if (flagWarning != null) flagWarning.push(string);
        this.getForwarder().warn(marker, string, thrwbl);
    }

    @Override
    public boolean isErrorEnabled() {
        return this.getForwarder().isErrorEnabled();
    }

    @Override
    public void error(String string) {
        if (flagError != null) flagError.push(string);
        this.getForwarder().error(string);
    }

    @Override
    public void error(String string, Object o) {
        if (flagError != null) flagError.push(string);
        this.getForwarder().error(string, o);
    }

    @Override
    public void error(String string, Object o, Object o1) {
        if (flagError != null) flagError.push(string);
        this.getForwarder().error(string, o, o1);
    }

    @Override
    public void error(String string, Object[] os) {
        if (flagError != null) flagError.push(string);
        this.getForwarder().error(string, os);
    }

    @Override
    public void error(String string, Throwable thrwbl) {
        if (flagError != null) flagError.push(string);
        this.getForwarder().error(string, thrwbl);
    }

    public void error(Throwable thrwbl) {
        String msg = thrwbl.getMessage();
        if (msg == null) msg = thrwbl.toString();
        if (flagError != null) flagError.push(msg);
        this.getForwarder().error(msg, thrwbl);
    }

    @Override
    public boolean isErrorEnabled(Marker marker) {
        return this.getForwarder().isErrorEnabled(marker);
    }

    @Override
    public void error(Marker marker, String string) {
        if (flagError != null) flagError.push(string);
        this.getForwarder().error(marker, string);
    }

    @Override
    public void error(Marker marker, String string, Object o) {
        if (flagError != null) flagError.push(string);
        this.getForwarder().error(marker, string, o);
    }

    @Override
    public void error(Marker marker, String string, Object o, Object o1) {
        if (flagError != null) flagError.push(string);
        this.getForwarder().error(marker, string, o, o1);
    }

    @Override
    public void error(Marker marker, String string, Object[] os) {
        if (flagError != null) flagError.push(string);
        this.getForwarder().error(marker, string, os);
    }

    @Override
    public void error(Marker marker, String string, Throwable thrwbl) {
        if (flagError != null) flagError.push(string);
        this.getForwarder().error(marker, string, thrwbl);
    }

    public static String pollWarningFlag() {
        StringBuilder ret = new StringBuilder();
        if (flagWarning != null) {
            for (;;) {
                String msg = flagWarning.pop();
                if (msg == null) break;
                ret.append(msg).append("\n");
            }
        }
        return ret.toString();
    }

    public static String pollErrorFlag() {
        StringBuilder ret = new StringBuilder();
        if (flagError != null) {
            for (;;) {
                String msg = flagError.pop();
                if (msg == null) break;
                ret.append(msg).append("\n");
            }
        }
        return ret.toString();
    }

    public static String pollWarningOrErrorFlag() {
        StringBuilder ret = new StringBuilder();
        ret.append(pollWarningFlag());
        ret.append(pollErrorFlag());
        return ret.toString();
    }

    public static void assertWarningOrErrorFlag() {
        String msgs = pollWarningOrErrorFlag();
        Assertions.assertFalse(msgs.length() > 0, msgs);
    }

    public static void resetWarningOrErrorFlag() {
        flagWarning = new ConcurrentStack<>();
        flagError = new ConcurrentStack<>();
    }

    public static void withNoWarningsOrErrors(Runnable f) {
        resetWarningOrErrorFlag();
        try {
            f.run();
            assertWarningOrErrorFlag();
        } finally {
            resetWarningOrErrorFlag();
        }
    }

    public static <A> void withNoWarningsOrErrors(Consumer<A> f, A a) {
        withNoWarningsOrErrors(() -> f.accept(a));
    }

    public static <A, B> void withNoWarningsOrErrors(BiConsumer<A, B> f, A a, B b) {
        withNoWarningsOrErrors(() -> f.accept(a, b));
    }

    public void redirect(HttpServletResponse response) {
        try {
            redirect(response.getOutputStream());
        } catch (IOException e) {
            error(e);
        }
    }

    public void redirect(OutputStream outputStream) {
        if (this.loggingDelegate != null) {
            this.loggingDelegate.redirect(outputStream);
        }
    }
}
