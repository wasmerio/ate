package com.tokera.ate.io.merge;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.google.common.collect.HashMultiset;
import com.tokera.ate.common.CopyOnWrite;
import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.CountLong;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.providers.PartitionKeySerializer;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.ApplicationScoped;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentMap;
import java.util.stream.Collectors;

/**
 * Class that will mergeThreeWay perform a 2-way mergeThreeWay on data objects, basic types and scales. All objects with read/write
 * properties and/or Column attribute marked fields will be in-scope of the mergeThreeWay.
 */
@SuppressWarnings({"unchecked"})
@ApplicationScoped
public class DataMerger {

    private static final ConcurrentMap<Class<?>, List<Field>> fieldDescriptorsMap = new ConcurrentHashMap<>();

    private boolean isInternal(Class<?> clazz) {
        if (clazz.isPrimitive() ||
            clazz.isSynthetic() ||
            clazz.isEnum()) {
            return true;
        }

        if (clazz == PUUID.class ||
            clazz == CountLong.class) {
            return true;
        }
        if (clazz == PartitionKeySerializer.PartitionKeyValue.class) {
            return true;
        }

        String name = clazz.getName();
        return name.startsWith("java.") ||
                name.startsWith("javax.") ||
                name.startsWith("com.sun.") ||
                name.startsWith("javax.") ||
                name.startsWith("oracle.");
    }

    private static boolean isDataField(Field field) {
        if (Modifier.isTransient(field.getModifiers()) == true) return false;
        if (field.getAnnotation(JsonIgnore.class) != null) return false;

        if (field.getAnnotation(JsonProperty.class) != null) return true;

        int modifiers = field.getModifiers();
        return Modifier.isPublic(modifiers);
    }

    private static List<Field> getAllFields(List<Field> fields, Class<?> type) {
        fields.addAll(Arrays.asList(type.getDeclaredFields()));

        if (type.getSuperclass() != null) {
            getAllFields(fields, type.getSuperclass());
        }

        return fields;
    }

    public static List<Field> getFieldDescriptors(Class<?> clazz) {
        return fieldDescriptorsMap
                .computeIfAbsent(clazz, (c) -> {
                    List<Field> fields = new ArrayList<>();
                    getAllFields(fields, clazz);
                    List<Field> ret = fields.stream()
                            .filter(p -> isDataField(p))
                            .collect(Collectors.toList());
                    ret.stream().forEach(f -> f.setAccessible(true));
                    return ret;
                });
    }

    @SuppressWarnings({"return.type.incompatible", "known.nonnull"})
    public Object newObject(Class<?> clazz) {
        try {
            return clazz.newInstance();
        } catch (InstantiationException | IllegalAccessException e) {
            throw new IllegalArgumentException("Class (" + clazz + ") must have a default constructor.", e);
        }
    }

    @SuppressWarnings({"argument.type.incompatible"})
    public @Nullable Object cloneObject(final @Nullable Object source) {
        if (source == null) return null;
        Class<?> clazz = source.getClass();

        if (isInternal(clazz)) {
            if (clazz == CountLong.class) return CountLong.clone(source);
            return source;
        }

        Object ret = newObject(clazz);
        if (ret instanceof CopyOnWrite) {
            ((CopyOnWrite) ret).copyOnWrite();
        }
        if (source instanceof Map) {
            return cloneObjectMap((Map) source, (Map) ret);
        } else if (source instanceof Collection) {
            return cloneObjectCollection((Collection) source, (Collection) ret);
        }
        cloneObjectFields(clazz, source, ret);
        return ret;
    }

    private Map cloneObjectMap(Map source, Map dest) {
        for (Object key : source.keySet()) {
            Object val = MapTools.getOrNull(source, key);
            dest.put(cloneObject(key), cloneObject(val));
        }
        return dest;
    }

    private Collection cloneObjectCollection(Collection source, Collection dest) {
        for (Object val : source) {
            dest.add(cloneObject(val));
        }
        return dest;
    }

    @SuppressWarnings({"argument.type.incompatible"})
    private Object cloneObjectFields(Class<?> clazz, Object source, Object dest) {
        List<Field> fields = this.getFieldDescriptors(clazz);
        for (Field field : fields) {
            try {
                Object val = field.get(source);
                field.set(dest, cloneObject(val));
            } catch (IllegalAccessException e) {
                throw new RuntimeException("Failed to set field", e);
            }
        }
        return dest;
    }

    private void mergeMapEntryThreeWay(Map ret, Object key, @Nullable Map common, @Nullable Map left, @Nullable Map right) {
        Object valCommon = null;
        Object valLeft = null;
        Object valRight = null;
        if (common != null) valCommon = common.getOrDefault(key, null);
        if (left != null) valLeft = left.getOrDefault(key, null);
        if (right != null) valRight = right.getOrDefault(key, null);
        Object val = mergeThreeWay(valCommon, valLeft, valRight);
        ret.put(key, val);
    }

    private void mergeMapThreeWay(Map ret, @Nullable Map common, @Nullable Map left, @Nullable Map right) {
        HashSet existsLeft;
        HashSet existsRight;
        if (left != null) existsLeft = new HashSet(left.keySet());
        else existsLeft = new HashSet();
        if (right != null) existsRight = new HashSet(right.keySet());
        else existsRight = new HashSet();

        if (common != null) {
            for (Object key : common.keySet()) {
                Object valCommon = MapTools.getOrNull(common, key);
                ret.put(key, cloneObject(valCommon));
            }
        }

        if (left != null) {
            left.keySet().stream().forEach(key -> mergeMapEntryThreeWay(ret, key, common, left, right));
        }
        if (right != null) {
            right.keySet().stream().forEach(key -> mergeMapEntryThreeWay(ret, key, common, left, right));
        }
        if (common != null) {
            common.keySet().stream()
                    .filter(e -> existsLeft.contains(e) == false ||
                            existsRight.contains(e) == false)
                    .forEach(key -> ret.remove(key));
        }
    }

    private void mergeMapApply(Map ret, @Nullable Map _base, @Nullable Map _what) {
        Map base = _base;
        Map what = _what;

        if (what == null) {
            if (base == null) return;
            for (Object key : base.keySet()) {
                ret.remove(key);
            }
            return;
        }
        if (base == null) {
            for (Object key : what.keySet()) {
                Object valWhat = what.get(key);
                Object valRet = MapTools.getOrNull(ret, key);
                if (valRet == null) valRet = valWhat;
                ret.put(key, mergeApply(null, valWhat, valRet));
            }
            return;
        }

        for (Object key : base.keySet()) {
            if (what.containsKey(key) == false) {
                ret.remove(key);
            }
        }
        for (Object key : what.keySet()) {
            Object valBase = MapTools.getOrNull(base, key);
            Object valWhat = what.get(key);
            Object valRet = MapTools.getOrNull(ret, key);
            if (valRet == null) valRet = valWhat;
            ret.put(key, mergeApply(valBase, valWhat, valRet));
        }
    }

    private void mergeSetThreeWay(Set ret, @Nullable Set common, @Nullable Set left, @Nullable Set right) {
        if (common != null) {
            common.stream().filter(val -> left.contains(val) == true && right.contains(val) == true)
                    .forEach(val -> ret.add(cloneObject(val)));
            if (left != null) {
                left.stream().filter(val -> common.contains(val) == false)
                        .forEach(val -> ret.add(cloneObject(val)));

                if (right != null) {
                    right.stream().filter(val -> common.contains(val) == false && left.contains(val) == false)
                            .forEach(val -> ret.add(cloneObject(val)));
                }
            } else {
                if (right != null) {
                    right.stream().filter(val -> common.contains(val) == false)
                            .forEach(val -> ret.add(cloneObject(val)));
                }
            }
        } else {
            if (left != null) {
                left.stream().forEach(val -> ret.add(cloneObject(val)));
                if (right != null) {
                    right.stream().filter(val -> left.contains(val) == false).forEach(val -> ret.add(cloneObject(val)));
                }
            } else {
                if (right != null) {
                    right.stream().forEach(val -> ret.add(cloneObject(val)));
                }
            }
        }
    }

    private void mergeCollectionThreeWay(Collection ret, @Nullable Collection common, @Nullable Collection left, @Nullable Collection right) {
        HashSet existsCommon;
        HashSet existsLeft;
        HashSet existsRight;
        if (common != null) existsCommon = new HashSet(common);
        else existsCommon = new HashSet();
        if (left != null) existsLeft = new HashSet(left);
        else existsLeft = new HashSet();
        if (right != null) existsRight = new HashSet(right);
        else existsRight = new HashSet();

        if (common != null) {
            common.stream().filter(val -> existsLeft.contains(val) == true && existsRight.contains(val) == true)
                    .forEach(val -> ret.add(cloneObject(val)));
        }
        if (left != null) {
            left.stream().filter(val -> existsCommon.contains(val) == false)
                    .forEach(val -> ret.add(cloneObject(val)));
        }

        if (right != null) {
            right.stream().filter(val -> existsCommon.contains(val) == false && existsLeft.contains(val) == false)
                    .forEach(val -> ret.add(cloneObject(val)));
        }
    }

    private void mergeSetApply(Set ret, @Nullable Set _base, @Nullable Set _what) {
        Set base = _base;
        Set what = _what;

        if (what == null) {
            if (base == null) return;
            for (Object val : base) {
                ret.remove(val);
            }
            return;
        }
        if (base == null) {
            for (Object val : what) {
                if (ret.contains(val) == false) {
                    ret.add(cloneObject(val));
                }
            }
            return;
        }
        for (Object val : base) {
            if (what.contains(val) == false) {
                ret.remove(val);
            }
        }
        for (Object val : what) {
            if (base.contains(val) == false) {
                ret.add(cloneObject(val));
            }
        }
    }

    private void mergeCollectionApply(Collection ret, @Nullable Collection _base, @Nullable Collection _what) {
        Collection base = _base;
        Collection what = _what;

        if (what == null) {
            if (base == null) return;
            for (Object val : base) {
                ret.remove(val);
            }
            return;
        }
        if (base == null) {
            HashMultiset<Object> existingRet = HashMultiset.create(ret);
            for (Object val : what) {
                if (existingRet.contains(val) == false) {
                    ret.add(cloneObject(val));
                }
            }
            return;
        }
        {
            HashMultiset<Object> existingWhat = HashMultiset.create(what);
            for (Object val : base) {
                if (existingWhat.contains(val) == false) {
                    ret.remove(val);
                }
            }
        }
        {
            HashMultiset<Object> existingBase = HashMultiset.create(base);
            for (Object val : what) {
                if (existingBase.contains(val) == false) {
                    ret.add(cloneObject(val));
                }
            }
        }
    }

    private void mergeListThreeWay(List ret, @Nullable List common, @Nullable List left, @Nullable List right) {
        HashSet existsCommon = new HashSet();
        HashSet existsLeft = new HashSet();
        HashSet existsRight = new HashSet();
        if (common != null) existsCommon = new HashSet(common);
        if (left != null) existsLeft = new HashSet(left);
        if (right != null) existsRight = new HashSet(right);

        if (common != null) {
            for (Object val : common) {
                ret.add(cloneObject(val));
            }
        }

        if (left != null) {
            for (int n = 0; n < left.size(); n++) {
                Object val = left.get(n);
                if (existsCommon.contains(val) == false) {
                    if (n + 1 == left.size()) {
                        ret.add(cloneObject(val));
                    } else if (n == 0) {
                        ret.add(0, cloneObject(val));
                    } else if (n < ret.size()) {
                        ret.add(n, cloneObject(val));
                    } else {
                        ret.add(cloneObject(val));
                    }
                }
            }
        }

        if (common != null) {
            for (Object val : common) {
                if (existsLeft.contains(val) == false) {
                    ret.remove(val);
                }
            }
        }

        if (right != null) {
            for (int n = 0; n < right.size(); n++) {
                Object val = right.get(n);
                if (existsCommon.contains(val) == false &&
                        existsLeft.contains(val) == false) {
                    if (n + 1 == right.size()) {
                        ret.add(cloneObject(val));
                    } else if (n == 0) {
                        ret.add(0, cloneObject(val));
                    } else if (n < ret.size()) {
                        ret.add(n, cloneObject(val));
                    } else {
                        ret.add(cloneObject(val));
                    }
                }
            }
        }

        if (common != null) {
            for (Object val : common) {
                if (existsRight.contains(val) == false) {
                    ret.remove(val);
                }
            }
        }
    }

    private void mergeListApply(List ret, @Nullable List _base, @Nullable List _what) {
        List base = _base;
        List what = _what;

        if (what == null) {
            if (base == null) return;
            for (Object val : base) {
                ret.remove(val);
            }
            return;
        }
        if (base == null) {
            HashMultiset<Object> existingRet = HashMultiset.create(ret);
            for (int n = 0; n < what.size(); n++) {
                Object val = what.get(n);
                if (existingRet.remove(val) == false) {
                    if (n + 1 == what.size()) {
                        ret.add(cloneObject(val));
                    } else if (n == 0) {
                        ret.add(0, cloneObject(val));
                    } else if (n < ret.size()) {
                        ret.add(n, cloneObject(val));
                    } else {
                        ret.add(cloneObject(val));
                    }
                }
            }
            return;
        }
        {
            HashMultiset<Object> existingWhat = HashMultiset.create(what);
            for (Object val : base) {
                if (existingWhat.remove(val) == false) {
                    ret.remove(val);
                }
            }
        }
        {
            HashMultiset<Object> existingBase = HashMultiset.create(base);
            for (int n = 0; n < what.size(); n++) {
                Object val = what.get(n);
                if (existingBase.remove(val) == false) {
                    if (n + 1 == what.size()) {
                        ret.add(cloneObject(val));
                    } else if (n == 0) {
                        ret.add(0, cloneObject(val));
                    } else if (n < ret.size()) {
                        ret.add(n, cloneObject(val));
                    } else {
                        ret.add(cloneObject(val));
                    }
                }
            }
        }
    }

    @SuppressWarnings({"argument.type.incompatible"})
    public <T> @Nullable T mergeThreeWay(final @Nullable T common, final @Nullable T left, final @Nullable T right) {

        // We will need to use reflection to mergeThreeWay these objects
        Class<?> clazzCommon = common != null ? common.getClass() : null;
        Class<?> clazzLeft = left != null ? left.getClass() : null;
        Class<?> clazzRight = right != null ? right.getClass() : null;

        // First compare the types and if they differ then switch them to the new type (with a bias towards the right)
        if (clazzCommon != null && clazzRight != null &&
                Objects.equals(clazzCommon, clazzRight) == false) {
            return (T)cloneObject(right);
        } else if (clazzCommon != null && clazzLeft != null &&
                Objects.equals(clazzCommon, clazzLeft) == false) {
            return (T)cloneObject(left);
        }

        // Make sure the clazz is set (or if its all null then just return null)
        if (clazzCommon == null) clazzCommon = clazzRight;
        if (clazzCommon == null) clazzCommon = clazzLeft;
        if (clazzCommon == null) return null;

        // If its a primative type then pick the right one (otherwise fall through)
        if (isInternal(clazzCommon))
        {
            // Maps and collections will be handled later
            if (Map.class.isAssignableFrom(clazzCommon) == false &&
                Collection.class.isAssignableFrom(clazzCommon) == false)
            {
                if (clazzCommon == CountLong.class) {
                    long b = common != null ? ((CountLong)common).longValue() : 0L;
                    long l = left != null ? ((CountLong)left).longValue() : 0L;
                    long r = right != null ? ((CountLong)right).longValue() : 0L;
                    return (T)new CountLong(b + (l-b) + (r-b));
                } else {
                    if (Objects.equals(common, right) == false) {
                        return (T) cloneObject(right);
                    } else if (Objects.equals(common, left) == false) {
                        return (T) cloneObject(left);
                    } else {
                        return (T) cloneObject(common);
                    }
                }
            }
        }

        // Check if its been nulled (if it has then simply return the null)
        if (right == null && common != null) {
            return null;
        } else if (left == null && common != null) {
            return null;
        }

        // Now attempt to create it (assuming it has a default constructor)
        Object ret = newObject(clazzCommon);

        if (common instanceof CopyOnWrite) {
            ((CopyOnWrite) common).copyOnWrite();
        }
        if (left instanceof CopyOnWrite) {
            ((CopyOnWrite) left).copyOnWrite();
        }
        if (right instanceof CopyOnWrite) {
            ((CopyOnWrite) right).copyOnWrite();
        }

        // If its a map then mergeThreeWay the entries
        if (ret instanceof Map) {
            mergeMapThreeWay((Map) ret, (Map) common, (Map) left, (Map) right);
            return (T)ret;
        }

        // If its a list then mergeThreeWay the entries
        if (ret instanceof List) {
            mergeListThreeWay((List) ret, (List) common, (List) left, (List) right);
            return (T)ret;
        }

        // If its a set then mergeThreeWay the entries
        if (ret instanceof Set) {
            mergeSetThreeWay((Set) ret, (Set) common, (Set) left, (Set) right);
            return (T)ret;
        }

        // If its a collection then mergeThreeWay the entries
        if (ret instanceof Collection) {
            mergeCollectionThreeWay((Collection) ret, (Collection) common, (Collection) left, (Collection) right);
            return (T)ret;
        }

        List<Field> fields = this.getFieldDescriptors(clazzCommon);
        for (Field field : fields) {
            try {
                Object valueCommon = null;
                Object valueLeft = null;
                Object valueRight = null;
                if (common != null) valueCommon = field.get(common);
                if (left != null) valueLeft = field.get(left);
                if (right != null) valueRight = field.get(right);

                Object value = mergeThreeWay(valueCommon, valueLeft, valueRight);
                field.set(ret, value);
            } catch (IllegalAccessException e) {
                throw new RuntimeException("Failed to set field", e);
            }
        }
        return (T)ret;
    }

    @SuppressWarnings({"argument.type.incompatible"})
    public <T> @Nullable T mergeApply(final @Nullable T _base, final @Nullable T _what, final @Nullable T _ret) {

        // If the new one is null then it should be null
        Object what = _what;
        if (what == null) {
            if (_base == null) return _ret;
            return null;
        }
        Class<?> clazz = what.getClass();

        // Validate the base object is of the correct type (if it is not then null it)
        Object base = _base;
        if (base != null && Objects.equals(base.getClass(), clazz) == false) base = null;

        // If the base and return values are null then we are already done
        if (base == null && _ret == null) {
            return (T)cloneObject(what);
        }

        // If its a primative type then we just clone the value and return it
        if (isInternal(clazz))
        {
            // Maps and collections will be handled later
            if (Map.class.isAssignableFrom(clazz) == false && Collection.class.isAssignableFrom(clazz) == false) {
                if (Objects.equals(_base, _what) == true) {
                    return _ret;
                } else {
                    return (T)cloneObject(what);
                }
            }
        }

        // Create the return object (of the correct type)
        Object ret = _ret;
        if (ret != null && Objects.equals(ret.getClass(), clazz) == false) ret = null;
        if (ret == null) ret = newObject(clazz);

        if (ret instanceof CopyOnWrite) {
            ((CopyOnWrite) ret).copyOnWrite();
        }
        if (base instanceof CopyOnWrite) {
            ((CopyOnWrite) base).copyOnWrite();
        }
        if (what instanceof CopyOnWrite) {
            ((CopyOnWrite) what).copyOnWrite();
        }

        // If its a map then mergeThreeWay the entries
        if (ret instanceof Map) {
            mergeMapApply((Map) ret, (Map) base, (Map) what);
            return (T)ret;
        }

        // If its a list then mergeThreeWay the entries
        if (ret instanceof List) {
            mergeListApply((List) ret, (List) base, (List) what);
            return (T)ret;
        }

        // If its a collection then mergeThreeWay the entries
        if (ret instanceof Set) {
            mergeSetApply((Set) ret, (Set) base, (Set) what);
            return (T)ret;
        }

        // If its a collection then mergeThreeWay the entries
        if (ret instanceof Collection) {
            mergeCollectionApply((Collection) ret, (Collection) base, (Collection) what);
            return (T)ret;
        }

        List<Field> fields = this.getFieldDescriptors(clazz);
        for (Field field : fields) {
            try {
                Object valueBase = base != null ? field.get(base) : null;
                Object valueWhat = field.get(what);
                Object valueRet = field.get(ret);

                Object value = mergeApply(valueBase, valueWhat, valueRet);
                field.set(ret, value);
            } catch (IllegalAccessException e) {
                throw new RuntimeException("Failed to set field", e);
            }
        }
        return (T)ret;
    }

    public <T> @Nullable T merge(MergeSet<T> set) {
        if (set.stream.isEmpty()) return set.first;
        T ret = set.first;
        T base = set.base;
        for (T right : set.stream) {
            if (right == null) {
                ret = null;
                continue;
            }
            ret = mergeThreeWay(base, ret, right);
            if (ret == null) throw new RuntimeException("Failed to mergeThreeWay data objects.");
        }
        return ret;
    }

    public <T> @Nullable T merge(@Nullable Iterable<MergePair<T>> stream) {
        if (stream == null) return null;
        T ret = null;
        for (MergePair<T> pair : stream) {
            ret = this.mergeApply(pair.base, pair.what, ret);
        }
        return ret;
    }
}
