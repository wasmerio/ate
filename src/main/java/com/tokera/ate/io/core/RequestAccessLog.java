package com.tokera.ate.io.core;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.units.ClassName;
import com.tokera.ate.units.DaoId;
import com.tokera.ate.units.Hash;

import javax.enterprise.context.RequestScoped;
import java.util.*;
import java.util.stream.Collectors;

/**
 * Log that holds reads and writes to objects during the scope of a currentRights
 * The primary use-case for this IO module is for cache-invalidation.
 */
@RequestScoped
public class RequestAccessLog {

    private final Map<@ClassName String, Integer> readClazzCnts = new HashMap<>();
    private final Map<@ClassName String, Integer> wroteClazzCnts = new HashMap<>();
    private final Set<String> readRecords = new HashSet<>();
    private final Set<String> wroteRecords = new HashSet<>();
    private boolean isPaused = false;

    private final int max_items_per_clazz = 10;
    
    public <T extends BaseDao> void recordRead(Class<T> clazz) {
        if (isPaused == true) return;
        String clazzName = clazz.getSimpleName();
        String clazzNameSep = clazzName + ":";
        
        Integer cnt = readClazzCnts.getOrDefault(clazzName, 0);
        if (cnt > 0 && cnt < Integer.MAX_VALUE) {
            readRecords.removeAll(readRecords.stream()
                    .filter(r -> r.startsWith(clazzNameSep))
                    .collect(Collectors.toSet()));
        }
        
        readRecords.add(clazzNameSep + "*");
        readClazzCnts.put(clazzName, Integer.MAX_VALUE);
    }

    public <T extends BaseDao> void recordWrote(Class<T> clazz) {
        if (isPaused == true) return;
        String clazzName = clazz.getSimpleName();
        String clazzNameSep = clazzName + ":";
        
        Integer cnt = wroteClazzCnts.getOrDefault(clazzName, 0);
        if (cnt > 0 && cnt < Integer.MAX_VALUE) {
            wroteRecords.removeAll(wroteRecords.stream()
                    .filter(r -> r.startsWith(clazzNameSep))
                    .collect(Collectors.toSet()));
        }
        
        wroteRecords.add(clazzNameSep + "*");
        wroteClazzCnts.put(clazzName, Integer.MAX_VALUE);
    }

    public void recordRead(@DaoId UUID id, Class<?> clazz) {
        if (isPaused == true) return;
        String clazzName = clazz.getSimpleName();
        String clazzNameSep = clazzName + ":";
        
        Integer cnt = readClazzCnts.getOrDefault(clazzName, 0);
        if (cnt >= max_items_per_clazz && cnt < Integer.MAX_VALUE) {
            readRecords.removeAll(readRecords.stream()
                    .filter(r -> r.startsWith(clazzNameSep))
                    .collect(Collectors.toSet()));
            
            readRecords.add(clazzNameSep + "*");
            readClazzCnts.put(clazzName, Integer.MAX_VALUE);
        }
        
        if (readRecords.add(clazz.getSimpleName() + ":" + id) == true) {
            readClazzCnts.put(clazzName, cnt + 1);
        }
    }

    public void recordWrote(@DaoId UUID id, Class<?> clazz) {
        if (isPaused == true) return;
        recordWrote(id, clazz.getSimpleName());
    }

    public void recordWrote(@DaoId UUID id, String clazzName) {
        if (isPaused == true) return;
        String clazzNameSep = clazzName + ":";

        Integer cnt = wroteClazzCnts.getOrDefault(clazzName, 0);
        if (cnt >= max_items_per_clazz && cnt < Integer.MAX_VALUE) {
            wroteRecords.removeAll(wroteRecords.stream()
                    .filter(r -> r.startsWith(clazzNameSep))
                    .collect(Collectors.toSet()));

            wroteRecords.add(clazzNameSep + "*");
            wroteClazzCnts.put(clazzName, Integer.MAX_VALUE);
        }

        if (wroteRecords.add(clazzName + ":" + id) == true) {
            wroteClazzCnts.put(clazzName, cnt + 1);
        }
    }
    
    public Set<@Hash String> getReadRecords() {
        return this.readRecords;
    }
    
    public Set<@Hash String> getWroteRecords() {
        return this.wroteRecords;
    }
    
    public void pause() {
        isPaused = true;
    }
    
    public void unpause() {
        isPaused = false;
    }
}
