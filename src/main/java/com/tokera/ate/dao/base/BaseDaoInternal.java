package com.tokera.ate.dao.base;

import com.tokera.ate.io.api.IPartitionKey;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.Set;
import java.util.UUID;

/**
 * This class is used to hide away the internals of ATE from those creating an using data objects, if there were an
 * equivalent of C# internal then I would have used this instead but java is not capable of this access modifier.
 */
public class BaseDaoInternal
{
    public static @Nullable IPartitionKey getPartitionKey(BaseDao obj) {
        return obj._partitionKey;
    }

    public static void setPartitionKey(BaseDao obj, IPartitionKey partitionKey) {
        obj._partitionKey = partitionKey;
    }

    public static UUID getVersion(BaseDao obj) {
        return obj._version;
    }

    public static void setVersion(BaseDao obj, UUID version) {
        obj._version = version;
    }

    public static UUID getPreviousVersion(BaseDao obj) {
        return obj._previousVersion;
    }

    public static void setPreviousVersion(BaseDao obj, @Nullable UUID version) {
        obj._previousVersion = version;
    }

    public static boolean getImmutable(BaseDao obj) {
        return obj._immutable;
    }

    public static void setImmutable(BaseDao obj, boolean immutable) {
        obj._immutable = immutable;
    }

    public static Set<UUID> getMergesVersions(BaseDao obj) {
        return obj._mergesVersions;
    }

    public static String getType(BaseDao obj) {
        return obj.getClass().getName();
    }

    public static String getShortType(BaseDao obj) {
        return obj.getClass().getSimpleName();
    }

    public static void setMergesVersions(BaseDao obj, Set<UUID> mergesVersions) {
        obj._mergesVersions = mergesVersions;
    }

    public static boolean hasSaved(BaseDao obj) {
        return obj.hasSaved();
    }

    public static void assertStillMutable(@Nullable BaseDao _entity) {
        BaseDao entity = _entity;
        if (entity == null) return;
        entity.assertStillMutable();
    }

    public static void newVersion(BaseDao obj) {
        obj.newVersion();
    }
}
