package com.tokera.ate.common;

import com.tokera.ate.dao.ILogable;
import com.tokera.ate.delegates.LoggingDelegate;

import javax.ws.rs.container.ContainerRequestContext;

import org.checkerframework.checker.nullness.qual.Nullable;
import org.slf4j.Marker;

import java.io.IOException;
import java.io.OutputStream;

/**
 * Custom logger that will batch the log results into a string builder attached to the currentRights currentRights rather
 * than directly to the log appender
 */
public class LoggerToRequest implements org.slf4j.Logger {

    private final org.slf4j.Logger baseLogger;
    private final LoggingDelegate logingDelegate;
    private final StringBuilder builder;
    private int stream_line_size = 0;

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
                    if (logable.getLog() != null) {
                        ctxSB.append(logable.getLog());
                    }
                    requestDelegate.getLogBuilderStdout().put(logable, ctxSB);
                }

                String prefix = requestDelegate.getLogPrefix();

                if (ctxSB.length() <= 0 || StringTools.endsWithNewline(ctxSB)) {
                    ctxSB.append(prefix);
                }
                ctxSB.append(value);

                logable.setLog(ctxSB.toString());
            }

            if (stderr) {
                StringBuilder ctxSB = MapTools.getOrNull(requestDelegate.getLogBuilderStderr(), logable);
                if (ctxSB == null) {
                    ctxSB = new StringBuilder();
                    if (logable.getError() != null) {
                        ctxSB.append(logable.getError());
                    }
                    requestDelegate.getLogBuilderStderr().put(logable, ctxSB);
                }

                String prefix = requestDelegate.getLogPrefix();

                if (ctxSB.length() <= 0 || StringTools.endsWithNewline(ctxSB)) {
                    ctxSB.append(prefix);
                }
                ctxSB.append(value);

                logable.setError(ctxSB.toString());
            }
        }
    }

    private void writeStream(Object value) {
        if (value == null) return;

        LoggingDelegate requestDelegate = this.logingDelegate;
        OutputStream stream = requestDelegate.getRedirectStream();
        if (stream != null) {
            try {
                if (stream_line_size > 0) {
                    stream.write(" ".getBytes());
                } else {
                    stream.write(requestDelegate.getLogPrefix().getBytes());
                }

                byte[] bytes = value.toString().getBytes();
                stream.write(bytes);
                stream_line_size += bytes.length;
            } catch (IOException e) {
            }
        }
    }

    public void flushStream() {
        LoggingDelegate requestDelegate = this.logingDelegate;
        OutputStream stream = requestDelegate.getRedirectStream();
        if (stream != null) {
            try {
                if (stream_line_size > 0) {
                    writeStream("\n");
                    stream.flush();
                }
            } catch (IOException e) {
            }
        }
        stream_line_size = 0;
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

        writeStream(string);
        flushStream();
    }

    @Override
    public void trace(String string, Object o) {
        write("trace: ");
        write(string);
        write(" [");
        write(o);
        write("]");
        write("\n");

        writeStream(string);
        writeStream(o);
        flushStream();
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

        writeStream(string);
        writeStream(o);
        writeStream(o1);
        flushStream();
    }

    @Override
    public void trace(String string, Object[] os) {
        write("trace: ");
        write(string);

        writeStream(string);

        for (Object o : os) {
            write(" [");
            write(o);
            write("]");

            writeStream(o);
        }
        write("\n");

        flushStream();
    }

    @Override
    public void trace(String string, Throwable thrwbl) {
        write("trace: ");
        write(string);
        write(" exception [");
        write(thrwbl);
        write("]");
        write("\n");

        writeStream(string);
        writeStream(thrwbl);
        flushStream();
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

        writeStream(string);
        writeStream(marker);
        flushStream();
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

        writeStream(string);
        writeStream(marker);
        writeStream(o);
        flushStream();
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

        writeStream(string);
        writeStream(marker);
        writeStream(o);
        writeStream(o1);
        flushStream();
    }

    @Override
    public void trace(Marker marker, String string, Object[] os) {
        write("trace: ");
        write(string);
        write(" marker=");
        write(marker.toString());

        writeStream(string);
        writeStream(marker);

        for (Object o : os) {
            write(" [");
            write(o);
            write("]");

            writeStream(o);
        }
        write("\n");

        flushStream();
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

        writeStream(string);
        writeStream(marker);
        writeStream(thrwbl);
        flushStream();
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

        writeStream(string);
        flushStream();
    }

    @Override
    public void debug(String string, Object o) {
        write("debug: ");
        write(string);
        write(" [");
        write(o);
        write("]");
        write("\n");

        writeStream(string);
        writeStream(o);
        flushStream();
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

        writeStream(string);
        writeStream(o);
        writeStream(o1);
        flushStream();
    }

    @Override
    public void debug(String string, Object[] os) {
        write("debug: ");
        write(string);

        writeStream(string);

        for (Object o : os) {
            write(" [");
            write(o);
            write("]");

            writeStream(o);
        }
        write("\n");

        flushStream();
    }

    @Override
    public void debug(String string, Throwable thrwbl) {
        write("debug: ");
        write(string);
        write(" exception [");
        write(thrwbl);
        write("]");
        write("\n");

        writeStream(string);
        writeStream(thrwbl);
        flushStream();
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

        writeStream(string);
        writeStream(marker);
        flushStream();
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

        writeStream(string);
        writeStream(marker);
        writeStream(o);
        flushStream();
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

        writeStream(string);
        writeStream(marker);
        writeStream(o);
        writeStream(o1);
        flushStream();
    }

    @Override
    public void debug(Marker marker, String string, Object[] os) {
        write("debug: ");
        write(string);
        write(" marker=");
        write(marker.toString());

        writeStream(string);
        writeStream(marker);

        for (Object o : os) {
            write(" [");
            write(o);
            write("]");

            writeStream(o);
        }
        write("\n");

        flushStream();
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

        writeStream(string);
        writeStream(marker);
        writeStream(thrwbl);
        flushStream();
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

        writeStream(string);
        flushStream();
    }

    @Override
    public void info(String string, Object o) {
        write("info: ");
        writeStdout(string);
        writeStdout(" [");
        writeStdout(o);
        writeStdout("]");
        writeStdout("\n");

        writeStream(string);
        writeStream(o);
        flushStream();
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

        writeStream(string);
        writeStream(o);
        writeStream(o1);
        flushStream();
    }

    @Override
    public void info(String string, Object[] os) {
        write("info: ");
        writeStdout(string);

        writeStream(string);

        for (Object o : os) {
            writeStdout(" [");
            writeStdout(o);
            writeStdout("]");

            writeStream(o);
        }
        writeStdout("\n");

        flushStream();
    }

    @Override
    public void info(String string, Throwable thrwbl) {
        write("info: ");
        writeStdout(string);
        writeStdout(" exception [");
        writeStdout(thrwbl);
        writeStdout("]");
        writeStdout("\n");

        writeStream(string);
        writeStream(thrwbl);
        flushStream();
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

        writeStream(string);
        writeStream(marker);
        flushStream();
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

        writeStream(string);
        writeStream(marker);
        writeStream(o);
        flushStream();
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

        writeStream(string);
        writeStream(marker);
        writeStream(o);
        writeStream(o1);
        flushStream();
    }

    @Override
    public void info(Marker marker, String string, Object[] os) {
        write("info: ");
        writeStdout(string);
        writeStdout(" marker=");
        writeStdout(marker.toString());

        writeStream(string);
        writeStream(marker);

        for (Object o : os) {
            writeStdout(" [");
            writeStdout(o);
            writeStdout("]");

            writeStream(o);
        }
        writeStdout("\n");

        flushStream();
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

        writeStream(string);
        writeStream(marker);
        writeStream(thrwbl);
        flushStream();
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

        writeStream(string);
        flushStream();
    }

    @Override
    public void warn(String string, Object o) {
        write("warn: ");
        writeStdwarn(string);
        writeStdwarn(" [");
        writeStdwarn(o);
        writeStdwarn("]");
        writeStdwarn("\n");

        writeStream(string);
        writeStream(o);
        flushStream();
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

        writeStream(string);
        writeStream(o);
        writeStream(o1);
        flushStream();
    }

    @Override
    public void warn(String string, Object[] os) {
        write("warn: ");
        writeStdwarn(string);

        writeStream(string);

        for (Object o : os) {
            writeStdwarn(" [");
            writeStdwarn(o);
            writeStdwarn("]");

            writeStream(o);
        }
        writeStdwarn("\n");

        flushStream();
    }

    @Override
    public void warn(String string, Throwable thrwbl) {
        write("warn: ");
        writeStdwarn(string);
        writeStdwarn(" exception [");
        writeStdwarn(thrwbl);
        writeStdwarn("]");
        writeStdwarn("\n");

        writeStream(string);
        writeStream(thrwbl);
        flushStream();
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

        writeStream(string);
        writeStream(marker);
        flushStream();
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

        writeStream(string);
        writeStream(marker);
        writeStream(o);
        flushStream();
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

        writeStream(string);
        writeStream(marker);
        writeStream(o);
        writeStream(o1);
        flushStream();
    }

    @Override
    public void warn(Marker marker, String string, Object[] os) {
        write("warn: ");
        writeStdwarn(string);
        writeStdwarn(" marker=");
        writeStdwarn(marker.toString());

        writeStream(string);
        writeStream(marker);

        for (Object o : os) {
            writeStdwarn(" [");
            writeStdwarn(o);
            writeStdwarn("]");

            writeStream(o);
        }
        writeStdwarn("\n");

        flushStream();
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

        writeStream(string);
        writeStream(marker);
        writeStream(thrwbl);
        flushStream();
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

        writeStream(string);
        flushStream();
    }

    @Override
    public void error(String string, Object o) {
        write("error: ");
        writeStderr(string);
        writeStderr(" [");
        writeStderr(o);
        writeStderr("]");
        writeStderr("\n");

        writeStream(string);
        writeStream(o);
        flushStream();
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

        writeStream(string);
        writeStream(o);
        writeStream(o1);
        flushStream();
    }

    @Override
    public void error(String string, Object[] os) {
        write("error: ");
        writeStderr(string);

        writeStream(string);

        for (Object o : os) {
            writeStderr(" [");
            writeStderr(o);
            writeStderr("]");

            writeStream(o);
        }
        writeStderr("\n");

        flushStream();
    }

    @Override
    public void error(String string, Throwable thrwbl) {
        write("error: ");
        writeStderr(string);
        writeStderr(" exception [");
        writeStderr(thrwbl);
        writeStderr("]");
        writeStderr("\n");

        writeStream(string);
        writeStream(thrwbl);
        flushStream();
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

        writeStream(string);
        writeStream(marker);
        flushStream();
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

        writeStream(string);
        writeStream(marker);
        writeStream(o);
        flushStream();
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

        writeStream(string);
        writeStream(marker);
        writeStream(o);
        writeStream(o1);
        flushStream();
    }

    @Override
    public void error(Marker marker, String string, Object[] os) {
        write("error: ");
        writeStderr(string);
        writeStderr(" marker=");
        writeStderr(marker.toString());

        writeStream(string);
        writeStream(marker);

        for (Object o : os) {
            writeStderr(" [");
            writeStderr(o);
            writeStderr("]");

            writeStream(o);
        }
        writeStderr("\n");

        flushStream();
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

        writeStream(string);
        writeStream(marker);
        writeStream(thrwbl);
        flushStream();
    }
}
