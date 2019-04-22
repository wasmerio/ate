package com.tokera.ate.common;

import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.Map;

/**
 * Used to work around problems with the nullability checker
 */
public class MapTools {

    /**
     * @return Returns the value held in the map referred by the key or returns null if it does not exist
     */
    @SuppressWarnings({"return.type.incompatible", "argument.type.incompatible"})
    public static <K, V> @Nullable V getOrNull(Map<K, V> map, @Nullable K key) {
        if (key == null) return null;
        return map.getOrDefault(key, null);
    }
}
