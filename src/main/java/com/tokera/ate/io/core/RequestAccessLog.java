package com.tokera.ate.io.core;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.units.ClassName;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;

import javax.enterprise.context.RequestScoped;
import java.util.*;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.stream.Collectors;

/**
 * Log that holds reads and writes to objects during the scope of a currentRights
 * The primary use-case for this IO module is for cache-invalidation.
 */
@RequestScoped
public class RequestAccessLog {

    private final LinkedHashSet<String> readRecords = new LinkedHashSet<>();
    private final LinkedHashSet<String> wroteRecords = new LinkedHashSet<>();
    private AtomicInteger pauseStack = new AtomicInteger(0);

    public static boolean enablePausing = false;

    private final int max_items = 50;

    public boolean shouldClip() {
        return readRecords.size() + wroteRecords.size() > max_items;
    }
    
    public <T extends BaseDao> void recordRead(Class<T> clazz) {
        if (pauseStack.get() > 0) return;
        String clazzName = clazz.getSimpleName();
        String clazzNameSep = clazzName + ":";

        if (shouldClip()) {
            readRecords.removeAll(readRecords.stream()
                    .filter(r -> r.startsWith(clazzNameSep))
                    .collect(Collectors.toSet()));
        }
        
        readRecords.add(clazzNameSep + "*");
    }

    public <T extends BaseDao> void recordWrote(Class<T> clazz) {
        if (pauseStack.get() > 0) return;
        String clazzName = clazz.getSimpleName();
        String clazzNameSep = clazzName + ":";

        if (shouldClip()) {
            wroteRecords.removeAll(wroteRecords.stream()
                    .filter(r -> r.startsWith(clazzNameSep))
                    .collect(Collectors.toSet()));
        }
        
        wroteRecords.add(clazzNameSep + "*");
    }

    public void recordRead(@DaoId UUID id, Class<? extends BaseDao> clazz) {
        if (pauseStack.get() > 0) return;
        recordRead(id, clazz.getSimpleName());
    }

    public void recordRead(@DaoId UUID id, String clazzName) {
        if (pauseStack.get() > 0) return;
        String clazzNameSep = clazzName + ":";

        if (shouldClip()) {
            readRecords.removeAll(readRecords.stream()
                    .filter(r -> r.startsWith(clazzNameSep))
                    .collect(Collectors.toSet()));
            readRecords.add(clazzNameSep + "*");
            return;
        }
        
        readRecords.add(clazzName + ":" + id);
    }

    public void recordWrote(@DaoId UUID id, Class<?> clazz) {
        if (pauseStack.get() > 0) return;
        recordWrote(id, clazz.getSimpleName());
    }

    public void recordWrote(@DaoId UUID id, String clazzName) {
        if (pauseStack.get() > 0) return;
        String clazzNameSep = clazzName + ":";

        if (shouldClip()) {
            wroteRecords.removeAll(wroteRecords.stream()
                    .filter(r -> r.startsWith(clazzNameSep))
                    .collect(Collectors.toSet()));

            wroteRecords.add(clazzNameSep + "*");
            return;
        }

        wroteRecords.add(clazzName + ":" + id);
    }
    
    public Set<@Hash String> getReadRecords() {
        return this.readRecords;
    }
    
    public Set<@Hash String> getWroteRecords() {
        return this.wroteRecords;
    }
    
    public void pause() {
        if (enablePausing) {
            pauseStack.incrementAndGet();
        }
    }
    
    public void unpause() {
        if (enablePausing) {
            pauseStack.decrementAndGet();
        }
    }
}
