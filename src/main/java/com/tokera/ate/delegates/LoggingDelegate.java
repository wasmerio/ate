package com.tokera.ate.delegates;

import com.tokera.ate.dao.ILogable;
import com.tokera.ate.units.DaoId;
import javax.enterprise.context.RequestScoped;
import java.util.*;

/**
 * Delegate used to interact with the logging engine, in particular it holds a StringBuilder that buffers the logs
 * for each currentRights scope.
 */
@RequestScoped
public class LoggingDelegate  {

    private final Stack<ILogable> logStack = new Stack<>();
    private final Map<ILogable, StringBuilder> logBuilderStdout = new HashMap<>();
    private final Map<ILogable, StringBuilder> logBuilderStderr = new HashMap<>();
    private String logPrefix = "";
    private StringBuilder loggingBuffer = new StringBuilder();

    public LoggingDelegate() {
    }

    /**
     * @return Stack of logable interfaces that are currently listening on the logs that might be written
     */
    public Stack<ILogable> getLogStack() {
        return logStack;
    }

    /**
     * @return The currentRights prefix to put in-front of all the lines that are logged to the loggers
     */
    public String getLogPrefix() {
        return logPrefix;
    }

    /**
     * Changes the logging prefix to a different fixed string
     */
    public void setLogPrefix(String logPrefix) {
        this.logPrefix = logPrefix;
    }

    /**
     * @return Gets a StringBuilder for a particular data object that can be used for StdOut buffering
     */
    public Map<ILogable, StringBuilder> getLogBuilderStdout() {
        return logBuilderStdout;
    }

    /**
     * @return Gets a StringBuilder for a particular data object that can be used for StdErr buffering
     */
    public Map<ILogable, StringBuilder> getLogBuilderStderr() {
        return logBuilderStderr;
    }

    /**
     * @return Returns a generic StringBuilder that buffers all the log output for the currentRights currentRights scope
     */
    public StringBuilder getLoggingBuffer() {
        return loggingBuffer;
    }
}