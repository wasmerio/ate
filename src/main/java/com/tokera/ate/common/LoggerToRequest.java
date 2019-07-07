package com.tokera.ate.common;

import com.tokera.ate.dao.ILogable;
import com.tokera.ate.delegates.LoggingDelegate;

import javax.ws.rs.container.ContainerRequestContext;

import org.checkerframework.checker.nullness.qual.Nullable;
import org.slf4j.Marker;

/**
 * Custom logger that will batch the log results into a string builder attached to the currentRights currentRights rather
 * than directly to the log appender
 */
public class LoggerToRequest implements org.slf4j.Logger {

    private final org.slf4j.Logger baseLogger;
    private final LoggingDelegate logingDelegate;
    private final StringBuilder builder;

    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible"})       // We want to return a null if the data does not exist and it must be atomic
    private static @Nullable Object getPropertyOrNull(ContainerRequestContext context, String s) {
        return context.getProperty(s);
    }

    public LoggerToRequest(org.slf4j.Logger baseLogger, LoggingDelegate requestDelegate) {
        this.baseLogger = baseLogger;
        this.logingDelegate = requestDelegate;
        this.builder = requestDelegate.getLoggingBuffer();
    }
    
    public void write(Object value) {
        write(value, false, false);
    }
    
    public void writeStdout(Object value) {
        write(value, true, false);
    }
    
    public void writeStderr(Object value) {
        write(value, false, true);
    }
    
    public void writeStdwarn(Object value) {
        write(value, true, true);
    }
    
    public void write(Object value, boolean stdout, boolean stderr) {
        StringBuilder sb = builder;
        sb.append(value);

        LoggingDelegate requestDelegate = this.logingDelegate;
        for (ILogable logable : requestDelegate.getLogStack())
        {
            if (stdout) {
                StringBuilder ctxSB = MapTools.getOrNull(requestDelegate.getLogBuilderStdout(), logable);
                if (ctxSB == null) {
                    ctxSB = new StringBuilder();
                    requestDelegate.getLogBuilderStdout().put(logable, ctxSB);
                }

                String prefix = requestDelegate.getLogPrefix();

                if (ctxSB.length() <= 0 || StringTools.endsWithNewline(ctxSB)) {
                    ctxSB.append(prefix);
                }
                ctxSB.append(prefix).append(value);

                logable.setLog(ctxSB.toString());
            }

            if (stderr) {
                StringBuilder ctxSB = MapTools.getOrNull(requestDelegate.getLogBuilderStderr(), logable);
                if (ctxSB == null) {
                    ctxSB = new StringBuilder();
                    requestDelegate.getLogBuilderStderr().put(logable, ctxSB);
                }

                String prefix = requestDelegate.getLogPrefix();

                if (ctxSB.length() <= 0 || StringTools.endsWithNewline(ctxSB)) {
                    ctxSB.append(prefix);
                }
                ctxSB.append(prefix).append(value);

                logable.setError(ctxSB.toString());
            }
        }
    }

    @Override
    public String getName() {
        return this.baseLogger.getName();
    }

    @Override
    public boolean isTraceEnabled() {
        return this.baseLogger.isTraceEnabled();
    }

    @Override
    public void trace(String string) {
        write("trace: ");
        write(string);
        write("\n");
    }

    @Override
    public void trace(String string, Object o) {
        write("trace: ");
        write(string);
        write(" [");
        write(o);
        write("]");
        write("\n");
    }

    @Override
    public void trace(String string, Object o, Object o1) {
        write("trace: ");
        write(string);
        write(" [");
        write(o);
        write("]");
        write(" [");
        write(o1);
        write("]");
        write("\n");
    }

    @Override
    public void trace(String string, Object[] os) {
        write("trace: ");
        write(string);
        for (Object o : os) {
            write(" [");
            write(o);
            write("]");
        }
        write("\n");
    }

    @Override
    public void trace(String string, Throwable thrwbl) {
        write("trace: ");
        write(string);
        write(" exception [");
        write(thrwbl);
        write("]");
        write("\n");
    }

    @Override
    public boolean isTraceEnabled(Marker marker) {
        return this.baseLogger.isTraceEnabled(marker);
    }

    @Override
    public void trace(Marker marker, String string) {
        write("trace: ");
        write(string);
        write(" marker=");
        write(marker.toString());
        write("\n");
    }

    @Override
    public void trace(Marker marker, String string, Object o) {
        write("trace: ");
        write(string);
        write(" marker=");
        write(marker.toString());
        write(" [");
        write(o);
        write("]");
        write("\n");
    }

    @Override
    public void trace(Marker marker, String string, Object o, Object o1) {
        write("trace: ");
        write(string);
        write(" marker=");
        write(marker.toString());
        write(" [");
        write(o);
        write("]");
        write(" [");
        write(o1);
        write("]");
        write("\n");
    }

    @Override
    public void trace(Marker marker, String string, Object[] os) {
        write("trace: ");
        write(string);
        write(" marker=");
        write(marker.toString());
        for (Object o : os) {
            write(" [");
            write(o);
            write("]");
        }
        write("\n");
    }

    @Override
    public void trace(Marker marker, String string, Throwable thrwbl) {
        write("trace: ");
        write(string);
        write(" marker=");
        write(marker.toString());
        write(" exception [");
        write(thrwbl);
        write("]");
        write("\n");
    }

    @Override
    public boolean isDebugEnabled() {
        return this.baseLogger.isDebugEnabled();
    }

    @Override
    public void debug(String string) {
        write("debug: ");
        write(string);
        write("\n");
    }

    @Override
    public void debug(String string, Object o) {
        write("debug: ");
        write(string);
        write(" [");
        write(o);
        write("]");
        write("\n");
    }

    @Override
    public void debug(String string, Object o, Object o1) {
        write("debug: ");
        write(string);
        write(" [");
        write(o);
        write("]");
        write(" [");
        write(o1);
        write("]");
        write("\n");
    }

    @Override
    public void debug(String string, Object[] os) {
        write("debug: ");
        write(string);
        for (Object o : os) {
            write(" [");
            write(o);
            write("]");
        }
        write("\n");
    }

    @Override
    public void debug(String string, Throwable thrwbl) {
        write("debug: ");
        write(string);
        write(" exception [");
        write(thrwbl);
        write("]");
        write("\n");
    }

    @Override
    public boolean isDebugEnabled(Marker marker) {
        return this.baseLogger.isDebugEnabled(marker);
    }

    @Override
    public void debug(Marker marker, String string) {
        write("debug: ");
        write(string);
        write(" marker=");
        write(marker.toString());
        write("\n");
    }

    @Override
    public void debug(Marker marker, String string, Object o) {
        write("debug: ");
        write(string);
        write(" marker=");
        write(marker.toString());
        write(" [");
        write(o);
        write("]");
        write("\n");
    }

    @Override
    public void debug(Marker marker, String string, Object o, Object o1) {
        write("debug: ");
        write(string);
        write(" marker=");
        write(marker.toString());
        write(" [");
        write(o);
        write("]");
        write(" [");
        write(o1);
        write("]");
        write("\n");
    }

    @Override
    public void debug(Marker marker, String string, Object[] os) {
        write("debug: ");
        write(string);
        write(" marker=");
        write(marker.toString());
        for (Object o : os) {
            write(" [");
            write(o);
            write("]");
        }
        write("\n");
    }

    @Override
    public void debug(Marker marker, String string, Throwable thrwbl) {
        write("debug: ");
        write(string);
        write(" marker=");
        write(marker.toString());
        write(" exception [");
        write(thrwbl);
        write("]");
        write("\n");
    }

    @Override
    public boolean isInfoEnabled() {
        return this.baseLogger.isInfoEnabled();
    }

    @Override
    public void info(String string) {
        write("info: ");
        writeStdout(string);
        if (StringTools.endsWithNewline(string) == false) {
            writeStdout("\n");
        }
    }

    @Override
    public void info(String string, Object o) {
        write("info: ");
        writeStdout(string);
        writeStdout(" [");
        writeStdout(o);
        writeStdout("]");
        writeStdout("\n");
    }

    @Override
    public void info(String string, Object o, Object o1) {
        write("info: ");
        writeStdout(string);
        writeStdout(" [");
        writeStdout(o);
        writeStdout("]");
        writeStdout(" [");
        writeStdout(o1);
        writeStdout("]");
        writeStdout("\n");
    }

    @Override
    public void info(String string, Object[] os) {
        write("info: ");
        writeStdout(string);
        for (Object o : os) {
            writeStdout(" [");
            writeStdout(o);
            writeStdout("]");
        }
        writeStdout("\n");
    }

    @Override
    public void info(String string, Throwable thrwbl) {
        write("info: ");
        writeStdout(string);
        writeStdout(" exception [");
        writeStdout(thrwbl);
        writeStdout("]");
        writeStdout("\n");
    }

    @Override
    public boolean isInfoEnabled(Marker marker) {
        return this.baseLogger.isInfoEnabled(marker);
    }

    @Override
    public void info(Marker marker, String string) {
        write("info: ");
        writeStdout(string);
        writeStdout(" marker=");
        writeStdout(marker.toString());
        writeStdout("\n");
    }

    @Override
    public void info(Marker marker, String string, Object o) {
        write("info: ");
        writeStdout(string);
        writeStdout(" marker=");
        writeStdout(marker.toString());
        writeStdout(" [");
        writeStdout(o);
        writeStdout("]");
        writeStdout("\n");
    }

    @Override
    public void info(Marker marker, String string, Object o, Object o1) {
        write("info: ");
        writeStdout(string);
        writeStdout(" marker=");
        writeStdout(marker.toString());
        writeStdout(" [");
        writeStdout(o);
        writeStdout("]");
        writeStdout(" [");
        writeStdout(o1);
        writeStdout("]");
        writeStdout("\n");
    }

    @Override
    public void info(Marker marker, String string, Object[] os) {
        write("info: ");
        writeStdout(string);
        writeStdout(" marker=");
        writeStdout(marker.toString());
        for (Object o : os) {
            writeStdout(" [");
            writeStdout(o);
            writeStdout("]");
        }
        writeStdout("\n");
    }

    @Override
    public void info(Marker marker, String string, Throwable thrwbl) {
        write("info: ");
        writeStdout(string);
        writeStdout(" marker=");
        writeStdout(marker.toString());
        writeStdout(" exception [");
        writeStdout(thrwbl);
        writeStdout("]");
        writeStdout("\n");
    }

    @Override
    public boolean isWarnEnabled() {
        return this.baseLogger.isWarnEnabled();
    }

    @Override
    public void warn(String string) {
        write("warn: ");
        writeStdwarn(string);
        if (StringTools.endsWithNewline(string) == false) {
            writeStdwarn("\n");
        }
    }

    @Override
    public void warn(String string, Object o) {
        write("warn: ");
        writeStdwarn(string);
        writeStdwarn(" [");
        writeStdwarn(o);
        writeStdwarn("]");
        writeStdwarn("\n");
    }

    @Override
    public void warn(String string, Object o, Object o1) {
        write("warn: ");
        writeStdwarn(string);
        writeStdwarn(" [");
        writeStdwarn(o);
        writeStdwarn("]");
        writeStdwarn(" [");
        writeStdwarn(o1);
        writeStdwarn("]");
        writeStdwarn("\n");
    }

    @Override
    public void warn(String string, Object[] os) {
        write("warn: ");
        writeStdwarn(string);
        for (Object o : os) {
            writeStdwarn(" [");
            writeStdwarn(o);
            writeStdwarn("]");
        }
        writeStdwarn("\n");
    }

    @Override
    public void warn(String string, Throwable thrwbl) {
        write("warn: ");
        writeStdwarn(string);
        writeStdwarn(" exception [");
        writeStdwarn(thrwbl);
        writeStdwarn("]");
        writeStdwarn("\n");
    }

    @Override
    public boolean isWarnEnabled(Marker marker) {
        return this.baseLogger.isWarnEnabled(marker);
    }

    @Override
    public void warn(Marker marker, String string) {
        write("warn: ");
        writeStdwarn(string);
        writeStdwarn(" marker=");
        writeStdwarn(marker.toString());
        writeStdwarn("\n");
    }

    @Override
    public void warn(Marker marker, String string, Object o) {
        write("warn: ");
        writeStdwarn(string);
        writeStdwarn(" marker=");
        writeStdwarn(marker.toString());
        writeStdwarn(" [");
        writeStdwarn(o);
        writeStdwarn("]");
        writeStdwarn("\n");
    }

    @Override
    public void warn(Marker marker, String string, Object o, Object o1) {
        write("warn: ");
        writeStdwarn(string);
        writeStdwarn(" marker=");
        writeStdwarn(marker.toString());
        writeStdwarn(" [");
        writeStdwarn(o);
        writeStdwarn("]");
        writeStdwarn(" [");
        writeStdwarn(o1);
        writeStdwarn("]");
        writeStdwarn("\n");
    }

    @Override
    public void warn(Marker marker, String string, Object[] os) {
        write("warn: ");
        writeStdwarn(string);
        writeStdwarn(" marker=");
        writeStdwarn(marker.toString());
        for (Object o : os) {
            writeStdwarn(" [");
            writeStdwarn(o);
            writeStdwarn("]");
        }
        writeStdwarn("\n");
    }

    @Override
    public void warn(Marker marker, String string, Throwable thrwbl) {
        write("warn: ");
        writeStdwarn(string);
        writeStdwarn(" marker=");
        writeStdwarn(marker.toString());
        writeStdwarn(" exception [");
        writeStdwarn(thrwbl);
        writeStdwarn("]");
        writeStdwarn("\n");
    }

    @Override
    public boolean isErrorEnabled() {
        return this.baseLogger.isErrorEnabled();
    }

    @Override
    public void error(String string) {
        write("error: ");
        writeStderr(string);
        if (StringTools.endsWithNewline(string) == false) {
            writeStderr("\n");
        }
    }

    @Override
    public void error(String string, Object o) {
        write("error: ");
        writeStderr(string);
        writeStderr(" [");
        writeStderr(o);
        writeStderr("]");
        writeStderr("\n");
    }

    @Override
    public void error(String string, Object o, Object o1) {
        write("error: ");
        writeStderr(string);
        writeStderr(" [");
        writeStderr(o);
        writeStderr("]");
        writeStderr(" [");
        writeStderr(o1);
        writeStderr("]");
        writeStderr("\n");
    }

    @Override
    public void error(String string, Object[] os) {
        write("error: ");
        writeStderr(string);
        for (Object o : os) {
            writeStderr(" [");
            writeStderr(o);
            writeStderr("]");
        }
        writeStderr("\n");
    }

    @Override
    public void error(String string, Throwable thrwbl) {
        write("error: ");
        writeStderr(string);
        writeStderr(" exception [");
        writeStderr(thrwbl);
        writeStderr("]");
        writeStderr("\n");
    }

    @Override
    public boolean isErrorEnabled(Marker marker) {
        return this.baseLogger.isErrorEnabled(marker);
    }

    @Override
    public void error(Marker marker, String string) {
        write("error: ");
        writeStderr(string);
        writeStderr(" marker=");
        writeStderr(marker.toString());
        writeStderr("\n");
    }

    @Override
    public void error(Marker marker, String string, Object o) {
        write("error: ");
        writeStderr(string);
        writeStderr(" marker=");
        writeStderr(marker.toString());
        writeStderr(" [");
        writeStderr(o);
        writeStderr("]");
        writeStderr("\n");
    }

    @Override
    public void error(Marker marker, String string, Object o, Object o1) {
        write("error: ");
        writeStderr(string);
        writeStderr(" marker=");
        writeStderr(marker.toString());
        writeStderr(" [");
        writeStderr(o);
        writeStderr("]");
        writeStderr(" [");
        writeStderr(o1);
        writeStderr("]");
        writeStderr("\n");
    }

    @Override
    public void error(Marker marker, String string, Object[] os) {
        write("error: ");
        writeStderr(string);
        writeStderr(" marker=");
        writeStderr(marker.toString());
        for (Object o : os) {
            writeStderr(" [");
            writeStderr(o);
            writeStderr("]");
        }
        writeStderr("\n");
    }

    @Override
    public void error(Marker marker, String string, Throwable thrwbl) {
        write("error: ");
        writeStderr(string);
        writeStderr(" marker=");
        writeStderr(marker.toString());
        writeStderr(" exception [");
        writeStderr(thrwbl);
        writeStderr("]");
        writeStderr("\n");
    }
}
