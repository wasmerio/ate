package com.tokera.ate.io;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.dao.base.BaseDaoInternal;
import com.tokera.ate.dao.enumerations.PermissionPhase;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.EffectivePermissions;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.exceptions.TransactionAbortedException;
import com.tokera.ate.io.api.*;
import com.tokera.ate.io.repo.DataSubscriber;
import com.tokera.ate.io.repo.DataTransaction;
import com.tokera.ate.io.task.PollHook;
import com.tokera.ate.qualifiers.BackendStorageSystem;
import com.tokera.ate.qualifiers.FrontendStorageSystem;
import com.tokera.ate.units.*;
import com.tokera.ate.io.repo.DataContainer;
import org.checkerframework.checker.nullness.qual.EnsuresNonNullIf;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import java.util.*;
import java.util.function.*;
import java.util.stream.Collectors;
import java.util.stream.Stream;

/**
 * Generic IO class used to access the IO subsystem without forcing it to be loaded before its initialized. Also
 * includes a bunch of built in helper classes that are best not placed in the interface itself
 */
@FrontendStorageSystem
@ApplicationScoped
public class HeadIO
{
    protected AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    @BackendStorageSystem
    protected IAteIO back;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    @BackendStorageSystem
    protected IPartitionResolver backPartitionResolver;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    @BackendStorageSystem
    protected IPartitionKeyMapper backPartitionKeyMapper;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    @BackendStorageSystem
    protected ISecurityCastleFactory backSecurityCastleFactory;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    @BackendStorageSystem
    protected ITokenParser backTokenParser;
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private LoggerHook LOG;
    private Random rand = new Random();

    public HeadIO() {
    }

    public void warm()
    {
        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();
        back.warm(partitionKey);
    }

    public void warmAndWait()
    {
        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();
        back.warmAndWait(partitionKey);
    }

    public void warm(IPartitionKey partitionKey) { back.warm(partitionKey); }

    public void warmAndWait(IPartitionKey partitionKey) { back.warmAndWait(partitionKey); }

    public void send(DataTransaction trans, boolean validate) { back.send(trans, validate); }

    /**
     * Flushes all the transactions to database
     */
    public void flushAll() {
        for (DataTransaction trans : d.requestContext.transactions()) {
            DataTransaction next = trans != d.requestContext.rootTransaction() ? d.requestContext.rootTransaction() : null;
            trans.flush(true, next);
        }
    }

    /**
     * Flushes all the transactions to database
     */
    public void flushAll(boolean validate) {
        for (DataTransaction trans : d.requestContext.transactions()) {
            DataTransaction next = trans != d.requestContext.rootTransaction() ? d.requestContext.rootTransaction() : null;
            trans.flush(validate, next);
        }
    }

    /**
     * Clears the current transaction
     */
    public void clear() {
        d.requestContext.currentTransaction().clear();
    }

    /**
     * Clears all the transaction including those on the stack
     */
    public void clearAll() {
        for (DataTransaction trans : d.requestContext.transactions()) {
            trans.clear();
        }
    }

    /**
     * Synchronize the current transaction
     */
    public void sync() {
        sync(d.requestContext.currentTransaction());
    }

    /**
     * Synchronizes all the partitions that were touched during the current transaction
     */
    public void sync(DataTransaction transaction)
    {
        Map<IPartitionKey, MessageSyncDto> syncs = new HashMap<>();
        for (IPartitionKey key : transaction.keys()) {
            syncs.put(key, beginSync(key));
        }
        for (Map.Entry<IPartitionKey, MessageSyncDto> pair : syncs.entrySet()) {
            finishSync(pair.getKey(), pair.getValue());
        }
    }

    public MessageSyncDto beginSync(IPartitionKey partitionKey)
    {
        MessageSyncDto sync = new MessageSyncDto(rand.nextLong(), rand.nextLong());
        return back.beginSync(partitionKey, sync);
    }

    public void finishSync(IPartitionKey partitionKey, MessageSyncDto sync)
    {
        back.finishSync(partitionKey, sync);
    }

    public void underTransaction(boolean sync, Runnable f)
    {
        DataTransaction trans = this.newTransaction(sync);
        try
        {
            f.run();
        } catch (Throwable ex) {
            trans.clear();
            throw new TransactionAbortedException(ex);
        } finally {
            completeTransaction(trans);
        }
    }

    public <A> void underTransaction(boolean sync, Consumer<A> f, A a) {
        underTransaction(sync, () -> f.accept(a));
    }

    public <A, B> void underTransaction(boolean sync, BiConsumer<A, B> f, A a, B b) {
        underTransaction(sync, () -> f.accept(a, b));
    }

    public <T> T underTransaction(boolean sync, Supplier<T> f)
    {
        DataTransaction trans = this.newTransaction(sync);
        try
        {
            return f.get();
        } catch (Throwable ex) {
            trans.clear();
            throw new TransactionAbortedException(ex);
        } finally {
            completeTransaction(trans);
        }
    }

    public <A, R> R underTransaction(boolean sync, Function<A, R> f, A a) {
        return underTransaction(sync, () -> f.apply(a));
    }

    public <A, B, R> R underTransaction(boolean sync, BiFunction<A, B, R> f, A a, B b) {
        return underTransaction(sync, () -> f.apply(a, b));
    }

    /**
     * Gets the current transaction thats in scope
     */
    public DataTransaction currentTransaction() {
        return d.requestContext.currentTransaction();
    }

    /**
     * Returns all the transactions currently tracked for this request
     */
    public Iterable<DataTransaction> transactions() {
        return d.requestContext.transactions();
    }

    /**
     * Starts a new transaction and puts it into stock
     */
    public DataTransaction newTransaction(boolean sync) {
        DataTransaction ret = new DataTransaction(sync);
        ret.copyCacheFrom(currentTransaction());
        d.requestContext.pushTransaction(ret);
        return ret;
    }

    /**
     * Completes the transaction and removes it from scope (if its still in scope that is
     * @param transaction
     */
    public void completeTransaction(DataTransaction transaction) {
        if (transaction == d.requestContext.rootTransaction()) {
            transaction.flush(true, null);
            return;
        }

        d.requestContext.removeTransaction(transaction);
        DataTransaction next = d.requestContext.currentTransaction();
        d.requestContext.pushTransaction(transaction);

        try {
            transaction.flush(true, next);
        } finally {
            d.requestContext.removeTransaction(transaction);
        }
    }

    public void withPartitionKey(IPartitionKey key, Runnable f)
    {
        d.requestContext.pushPartitionKey(key);
        try { f.run(); }
        finally { d.requestContext.popPartitionKey(); }
    }

    /**
     * Commits the current transaction and waits for the object to change ignoring any changes that result
     * from the commit of the transaction itself
     * @return True if a change was detected to the data object within the 30 seconds
     */
    public <T extends BaseDao> T poll(PUUID id, Class<T> clazz) {
        return poll(id, clazz, 30000);
    }

    /**
     * Commits the current transaction and waits for the object to change ignoring any changes that result
     * from the commit of the transaction itself
     * @param timeout Time in milliseconds before the poll request will timeout
     * @return True if a change was detected to the data object within the timeout
     */
    @SuppressWarnings("unchecked")
    public <T extends BaseDao> T poll(PUUID id, Class<T> clazz, long timeout)
    {
        IPartitionKey partitionKey = id.partition();

        PollHook pollHook = new PollHook(id, clazz);
        try {
            d.hookManager.hook(partitionKey, clazz, pollHook);
            currentTransaction().flush(true,  null);
            return (T)pollHook.poll(timeout);
        } finally {
            d.hookManager.unhook(pollHook, clazz);
        }
    }

    public <A> void withPartitionKey(IPartitionKey key, Consumer<A> f, A a) {
        withPartitionKey(key, () -> f.accept(a));
    }

    public <A, B> void withPartitionKey(IPartitionKey key, BiConsumer<A, B> f, A a, B b) {
        withPartitionKey(key, () -> f.accept(a, b));
    }

    public <T> T withPartitionKey(IPartitionKey key, Supplier<T> f)
    {
        d.requestContext.pushPartitionKey(key);
        try { return f.get(); }
        finally { d.requestContext.popPartitionKey(); }
    }

    public <A, R> R withPartitionKey(IPartitionKey key, Function<A, R> f, A a) {
        return withPartitionKey(key, () -> f.apply(a));
    }

    public <A, B, R> R withPartitionKey(IPartitionKey key, BiFunction<A, B, R> f, A a, B b) {
        return withPartitionKey(key, () -> f.apply(a, b));
    }

    public void withPartitionKey(BaseDao from, Runnable f)
    {
        IPartitionKey key = from.partitionKey(true);
        d.requestContext.pushPartitionKey(key);
        try { f.run(); }
        finally { d.requestContext.popPartitionKey(); }
    }

    public <A> void withPartitionKey(BaseDao from, Consumer<A> f, A a) {
        IPartitionKey key = from.partitionKey(true);
        withPartitionKey(key, () -> f.accept(a));
    }

    public <A, B> void withPartitionKey(BaseDao from, BiConsumer<A, B> f, A a, B b) {
        IPartitionKey key = from.partitionKey(true);
        withPartitionKey(key, () -> f.accept(a, b));
    }

    public <T> T withPartitionKey(BaseDao from, Supplier<T> f)
    {
        IPartitionKey key = from.partitionKey(true);
        d.requestContext.pushPartitionKey(key);
        try { return f.get(); }
        finally { d.requestContext.popPartitionKey(); }
    }

    public <A, R> R withPartitionKey(BaseDao from, Function<A, R> f, A a) {
        IPartitionKey key = from.partitionKey(true);
        return withPartitionKey(key, () -> f.apply(a));
    }

    public <A, B, R> R withPartitionKey(BaseDao from, BiFunction<A, B, R> f, A a, B b) {
        IPartitionKey key = from.partitionKey(true);
        return withPartitionKey(key, () -> f.apply(a, b));
    }

    public @Nullable IPartitionKey currentPartitionKey() {
        return d.requestContext.currentPartitionKey();
    }

    public IPartitionResolver partitionResolver() {
        return this.backPartitionResolver;
    }

    public IPartitionKeyMapper partitionKeyMapper() { return this.backPartitionKeyMapper; }

    public void underTransactionWithPartitionKey(boolean sync, IPartitionKey key, Runnable f)
    {
        d.requestContext.pushPartitionKey(key);
        try {
            DataTransaction trans = this.newTransaction(sync);
            try {
                f.run();
            } catch (Throwable ex) {
                trans.clear();
                throw new TransactionAbortedException(ex);
            } finally {
                completeTransaction(trans);
            }
        } finally {
            d.requestContext.popPartitionKey();
        }
    }

    public <A> void underTransactionWithPartitionKey(boolean sync, IPartitionKey key, Consumer<A> f, A a) {
        underTransactionWithPartitionKey(sync, key, () -> f.accept(a));
    }

    public <A, B> void underTransactionWithPartitionKey(boolean sync, IPartitionKey key, BiConsumer<A, B> f, A a, B b) {
        underTransactionWithPartitionKey(sync, key, () -> f.accept(a, b));
    }

    public <T> T underTransactionWithPartitionKey(boolean sync, IPartitionKey key, Supplier<T> f)
    {
        d.requestContext.pushPartitionKey(key);
        try {
            DataTransaction trans = this.newTransaction(sync);
            try {
                return f.get();
            } catch (Throwable ex) {
                trans.clear();
                throw new TransactionAbortedException(ex);
            } finally {
                completeTransaction(trans);
            }
        } finally {
            d.requestContext.popPartitionKey();
        }
    }

    public <A, R> R underTransactionWithPartitionKey(boolean sync, IPartitionKey key, Function<A, R> f, A a) {
        return underTransactionWithPartitionKey(sync, key, () -> f.apply(a));
    }

    public <A, B, R> R underTransactionWithPartitionKey(boolean sync, IPartitionKey key, BiFunction<A, B, R> f, A a, B b) {
        return underTransactionWithPartitionKey(sync, key, () -> f.apply(a, b));
    }

    public ISecurityCastleFactory securityCastleFactory() {
        return this.backSecurityCastleFactory;
    }
    
    public ITokenParser tokenParser() { return this.backTokenParser; }

    public @Nullable MessagePublicKeyDto publicKeyOrNull(@Hash String hash) {
        IPartitionKey partitionKey = d.requestContext.getPartitionKeyScopeOrNull();
        if (partitionKey != null) {
            @Nullable MessagePublicKeyDto ret = this.publicKeyOrNull(partitionKey, hash);
            if (ret != null) return ret;
        }
        for (IPartitionKey otherKey : d.requestContext.getOtherPartitionKeys()) {
            @Nullable MessagePublicKeyDto ret = this.publicKeyOrNull(otherKey, hash);
            if (ret != null) return ret;
        }
        return null;
    }

    public @Nullable MessagePublicKeyDto publicKeyOrNull(IPartitionKey partitionKey, @Nullable @Hash String _hash) {
        @Hash String hash = _hash;
        if (hash == null) return null;
        return back.publicKeyOrNull(partitionKey, hash);
    }

    public boolean exists(@Nullable @DaoId UUID _id) {
        @DaoId UUID id = _id;
        if (id == null) return false;

        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();
        if (currentTransaction().exists(partitionKey, id)) {
            return true;
        }

        return back.exists(PUUID.from(partitionKey, id));
    }

    @EnsuresNonNullIf(expression="#1", result=true)
    public boolean exists(IPartitionKey partitionKey, @DaoId UUID id) {
        if (currentTransaction().exists(partitionKey, id)) {
            return true;
        }

        return back.exists(PUUID.from(partitionKey, id));
    }

    @EnsuresNonNullIf(expression="#1", result=true)
    public boolean exists(@Nullable PUUID id) {
        if (id == null) return false;

        if (currentTransaction().exists(id.partition(), id.id())) {
            return true;
        }

        return back.exists(id);
    }

    public boolean everExisted(@Nullable @DaoId UUID _id) {
        @DaoId UUID id = _id;
        if (id == null) return false;

        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();
        if (currentTransaction().exists(partitionKey, id)) {
            return true;
        }

        return back.everExisted(PUUID.from(partitionKey, id));
    }

    public boolean everExisted(@Nullable PUUID id){
        if (id == null) return false;

        if (currentTransaction().exists(id.partition(), id.id())) {
            return true;
        }

        return back.everExisted(id);
    }

    public boolean immutable(@DaoId UUID id) {
        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();
        return back.immutable(PUUID.from(partitionKey, id));
    }

    public boolean immutable(PUUID id) {
        return back.immutable(id);
    }

    public @Nullable MessageDataHeaderDto getRootOfTrust(@DaoId UUID id) {
        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();
        return back.readRootOfTrust(PUUID.from(partitionKey, id));
    }

    public @Nullable MessageDataHeaderDto readRootOfTrust(PUUID id) {
        return back.readRootOfTrust(id);
    }

    public @Nullable BaseDao readOrNull(@DaoId UUID id) {
        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();

        if (currentTransaction().findSavedDelete(partitionKey, id)) {
            return null;
        }

        BaseDao ret = currentTransaction().find(partitionKey, id);
        if (ret != null) return ret;

        return back.readOrNull(PUUID.from(partitionKey, id));
    }

    public @Nullable BaseDao readOrNull(PUUID id) {
        IPartitionKey partitionKey = id.partition();

        if (currentTransaction().findSavedDelete(partitionKey, id.id())) {
            return null;
        }

        BaseDao ret = currentTransaction().find(partitionKey, id.id());
        if (ret != null) return ret;

        ret = back.readOrNull(id);

        if (ret != null) {
            currentTransaction().cache(partitionKey, ret);
        }

        return ret;
    }

    public <T extends BaseDao> T read(IPartitionKey partitionKey, @DaoId UUID id, Class<T> type) {
        return read(PUUID.from(partitionKey, id), type);
    }

    public <T extends BaseDao> T read(@DaoId UUID id, Class<T> type) {
        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();
        return this.read(PUUID.from(partitionKey, id), type);
    }

    public BaseDao readOrThrow(PUUID id) {
        IPartitionKey partitionKey = id.partition();

        if (currentTransaction().findSavedDelete(id.partition(), id.id())) {
            throw new RuntimeException("This object has been removed according to the transaction logs [" + id + "].");
        }

        BaseDao ret = currentTransaction().find(partitionKey, id.id());
        if (ret != null) return ret;

        ret = back.readOrThrow(id);

        if (ret != null) {
            currentTransaction().cache(partitionKey, ret);
        }

        return ret;
    }

    @SuppressWarnings({"unchecked"})
    public <T extends BaseDao> T read(PUUID id, Class<T> type) {
        try {
            BaseDao ret = this.readOrThrow(id);
            if (ret != null) {
                currentTransaction().cache(id.partition(), ret);
            }
            if (ret == null) {
                throw new RuntimeException(type.getSimpleName() + " not found (id=" + id.print() + ")");
            }
            if (ret.getClass() != type) {
                throw new RuntimeException(type.getSimpleName() + " of the wrong type (id=" + id.print() + ", actual=" + ret.getClass().getSimpleName() + ", expected=" + type.getSimpleName() + ")");
            }
            BaseDaoInternal.assertStillMutable(ret);
            return (T)ret;
        } catch (ClassCastException ex) {
            throw new RuntimeException(type.getSimpleName() + " of the wrong type (id=" + id.print() + ")", ex);
        }
    }

    protected BaseDao read(@DaoId UUID id) {
        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();
        return this.read(PUUID.from(partitionKey, id));
    }

    protected BaseDao read(PUUID id) {
        BaseDao ret = this.readOrThrow(id);
        if (ret == null) {
            throw new RuntimeException("Object data (id=" + id.print() + ") not found");
        }

        currentTransaction().cache(id.partition(), ret);
        return ret;
    }

    public DataContainer readRaw(@DaoId UUID id)
    {
        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();
        return this.readRaw(PUUID.from(partitionKey, id));
    }

    public DataContainer readRaw(PUUID id)
    {
        DataContainer ret = back.readRawOrNull(id);
        if (ret == null) {
            throw new RuntimeException("Object data (id=" + id.print() + ") not found");
        }
        return ret;
    }

    public @Nullable DataContainer readRawOrNull(@DaoId UUID id)
    {
        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();
        return back.readRawOrNull(PUUID.from(partitionKey, id));
    }

    public @Nullable DataContainer readRawOrNull(PUUID id)
    {
        return back.readRawOrNull(id);
    }

    public <T extends BaseDao> Iterable<MessageMetaDto> readHistory(@DaoId UUID id, Class<T> clazz) {
        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();
        return back.readHistory(PUUID.from(partitionKey, id), clazz);
    }

    public <T extends BaseDao> Iterable<MessageMetaDto> readHistory(PUUID id, Class<T> clazz) {
        return back.readHistory(id, clazz);
    }

    public @Nullable BaseDao readVersionOrNull(@DaoId UUID id, long offset) {
        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();
        return back.readVersionOrNull(PUUID.from(partitionKey, id), offset);
    }

    public @Nullable BaseDao readVersionOrNull(PUUID id, long offset) {
        return back.readVersionOrNull(id, offset);
    }

    public @Nullable MessageDataMetaDto readVersionMsgOrNull(@DaoId UUID id, long offset) {
        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();
        return back.readVersionMsgOrNull(PUUID.from(partitionKey, id), offset);
    }

    public @Nullable MessageDataMetaDto readVersionMsgOrNull(PUUID id, long offset) {
        return back.readVersionMsgOrNull(id, offset);
    }

    public BaseDao readVersion(@DaoId UUID id, long offset) {
        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();
        return this.readVersion(PUUID.from(partitionKey, id), offset);
    }

    public BaseDao readVersion(PUUID id, long offset) {
        BaseDao ret = back.readVersionOrNull(id, offset);
        if (ret == null) {
            throw new RuntimeException("Object version data (id=" + id.print() + ") not found");
        }
        return ret;
    }

    public MessageDataMetaDto readVersionMsg(@DaoId UUID id, long offset) {
        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();
        return this.readVersionMsg(PUUID.from(partitionKey, id), offset);
    }

    public MessageDataMetaDto readVersionMsg(PUUID id, long offset) {
        MessageDataMetaDto ret = back.readVersionMsgOrNull(id, offset);
        if (ret == null) {
            throw new RuntimeException("Object version message (id=" + id.print() + ") not found");
        }
        return ret;
    }

    @SuppressWarnings("unchecked")
    public <T extends BaseDao> boolean test(PUUID key, Class<T> type, Predicate<T> predicate) {
        EffectivePermissions perms = d.permissionCache.perms(type.getName(), key.partition(), key.id(), PermissionPhase.BeforeMerge);
        if (perms.canRead(d.currentRights)) {
            BaseDao obj = this.readOrNull(key);
            if (obj != null) {
                return predicate.test((T)obj);
            }
        }
        return false;
    }

    /**
     * Produces a materialized view of the data objects visible by this context
     */

    public <T extends BaseDao> Stream<T> view(IPartitionKey key, Class<T> type, Predicate<T> predicate) {
        return back.view(key, type, predicate).stream();
    }

    public <T extends BaseDao> List<T> viewAsList(IPartitionKey key, Class<T> type, Predicate<T> predicate) {
        return back.view(key, type, predicate);
    }

    public <T extends BaseDao> Set<T> viewAsSet(IPartitionKey key, Class<T> type, Predicate<T> predicate) {
        return back.view(key, type, predicate).stream().collect(Collectors.toSet());
    }

    public <T extends BaseDao, K, V> Map<K, V> viewAsMap(IPartitionKey key, Class<T> type, Predicate<T> predicate, Function<T, K> mapKey, Function<T, V> mapVal) {
        return back.view(key, type, predicate).stream().collect(Collectors.toMap(mapKey, mapVal, (a, b) -> b));
    }

    public <T extends BaseDao> Stream<T> view(Class<T> type, Predicate<T> predicate) {
        return back.view(this.currentPartitionKey(), type, predicate).stream();
    }

    public <T extends BaseDao> List<T> viewAsList(Class<T> type, Predicate<T> predicate) {
        return back.view(this.currentPartitionKey(), type, predicate);
    }

    public <T extends BaseDao> Set<T> viewAsSet(Class<T> type, Predicate<T> predicate) {
        return back.view(this.currentPartitionKey(), type, predicate).stream().collect(Collectors.toSet());
    }

    public <T extends BaseDao, K, V> Map<K, V> viewAsMap(Class<T> type, Predicate<T> predicate, Function<T, K> mapKey, Function<T, V> mapVal) {
        return back.view(this.currentPartitionKey(), type, predicate).stream().collect(Collectors.toMap(mapKey, mapVal, (a, b) -> b));
    }

    public <T extends BaseDao> Stream<T> view(Collection<IPartitionKey> keys, Class<T> type, Predicate<T> predicate) {
        return keys.stream().flatMap(k -> view(k, type, predicate));
    }

    public <T extends BaseDao> List<T> viewAsList(Collection<IPartitionKey> keys, Class<T> type, Predicate<T> predicate) {
        return keys.stream().flatMap(k -> view(k, type, predicate)).collect(Collectors.toList());
    }

    public <T extends BaseDao> Set<T> viewAsSet(Collection<IPartitionKey> keys, Class<T> type, Predicate<T> predicate) {
        return keys.stream().flatMap(k -> view(k, type, predicate)).collect(Collectors.toSet());
    }

    public <T extends BaseDao, K, V> Map<K, V> viewAsMap(Collection<IPartitionKey> keys, Class<T> type, Predicate<T> predicate, Function<T, K> mapKey, Function<T, V> mapVal) {
        return keys.stream().flatMap(k -> view(k, type, predicate)).collect(Collectors.toMap(mapKey, mapVal, (a, b) -> b));
    }

    /** Inner joins **/

    public <T extends BaseDao> Stream<T> join(PUUID key, Class<T> clazz, Function<T, UUID> joiningField) {
        return d.indexing.join(key, clazz, joiningField).stream();
    }

    public <T extends BaseDao> List<T> joinAsList(PUUID key, Class<T> clazz, Function<T, UUID> joiningField) {
        return d.indexing.join(key, clazz, joiningField);
    }

    public <T extends BaseDao> Set<T> joinAsSet(PUUID key, Class<T> clazz, Function<T, UUID> joiningField) {
        return d.indexing.join(key, clazz, joiningField).stream().collect(Collectors.toSet());
    }

    public <T extends BaseDao, K, V> Map<K, V> joinAsMap(PUUID key, Class<T> clazz, Function<T, UUID> joiningField, Function<T, K> mapKey, Function<T, V> mapVal) {
        return d.indexing.join(key, clazz, joiningField).stream()
                .collect(Collectors.toMap(mapKey, mapVal, (a, b) -> b));
    }

    /** Full table scans **/

    public List<BaseDao> all() {
        return all(d.requestContext.currentPartitionKey());
    }

    public List<BaseDao> all(IPartitionKey partitionKey) {
        List<BaseDao> ret = back.view(partitionKey, a -> true);
        for (BaseDao entity : ret) {
            currentTransaction().cache(partitionKey, entity);
        }
        return ret;
    }

    public <T extends BaseDao> List<T> all(Collection<IPartitionKey> keys, Class<T> type) {
        return keys.stream().flatMap(k -> all(k, type).stream()).collect(Collectors.toList());
    }

    public <T extends BaseDao> List<T> all(Class<T> type) {
        return all(d.requestContext.currentPartitionKey(), type);
    }

    public <T extends BaseDao> List<T> all(IPartitionKey partitionKey, Class<T> type) {
        List<T> ret = back.view(partitionKey, type, a -> true);
        for (BaseDao entity : ret) {
            currentTransaction().cache(partitionKey, entity);
        }
        return ret;
    }

    public <T extends BaseDao> List<DataContainer> readAllRaw()
    {
        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();
        return back.readAllRaw(partitionKey);
    }

    public <T extends BaseDao> List<DataContainer> readAllRaw(IPartitionKey partitionKey) { return back.readAllRaw(partitionKey); }

    public <T extends BaseDao> List<DataContainer> readAllRaw(Class<T> type)
    {
        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();
        return back.readAllRaw(partitionKey, type);
    }

    public <T extends BaseDao> List<DataContainer> readAllRaw(IPartitionKey partitionKey, Class<T> type) { return back.readAllRaw(partitionKey, type); }

    public List<BaseDao> readOrNull(Iterable<UUID> ids) {
        IPartitionKey partitionKey = d.requestContext.currentPartitionKey();
        return this.readOrNull(partitionKey, ids);
    }

    public List<BaseDao> readOrNull(IPartitionKey partitionKey, Iterable<UUID> ids) {
        List<BaseDao> ret = new ArrayList<>();
        for (UUID id : ids) {
            BaseDao entity = this.readOrNull(PUUID.from(partitionKey, id));
            if (entity != null) {
                ret.add(entity);
            }
        }
        return ret;
    }

    public List<BaseDao> readOrNull(Collection<PUUID> ids) {
        ids.stream().forEach(id -> this.warm(id.partition()));

        ArrayList<BaseDao> ret = new ArrayList<>();
        for (PUUID id : ids) {
            ret.add(this.readOrNull(id));
        }
        return ret;
    }

    public <T extends BaseDao> List<T> read(Iterable<UUID> ids, Class<T> type) {
        return this.read(d.requestContext.currentPartitionKey(), ids, type);
    }

    public <T extends BaseDao> List<T> read(IPartitionKey partitionKey, Iterable<UUID> ids, Class<T> type) {
        ArrayList<T> ret = new ArrayList<>();
        for (UUID id : ids) {
            ret.add(this.read(PUUID.from(partitionKey, id), type));
        }
        return ret;
    }

    public <T extends BaseDao> List<T> readAccessible(IPartitionKey partitionKey, Iterable<UUID> ids, Class<T> type) {
        ArrayList<T> ret = new ArrayList<>();
        for (UUID id : ids) {
            if (this.exists(partitionKey, id) == false) continue;
            ret.add(this.read(PUUID.from(partitionKey, id), type));
        }
        return ret;
    }

    public <T extends BaseDao> List<T> read(Collection<PUUID> ids, Class<T> type) {
        ids.stream().forEach(id -> this.warm(id.partition()));

        ArrayList<T> ret = new ArrayList<>();
        for (PUUID id : ids) {
            ret.add(this.read(id, type));
        }
        return ret;
    }

    public DataSubscriber backend() {
        return back.backend();
    }

    /**
     * Writes a data object to this transaction which will be commited to the database along with the whole transaction
     */
    public void write(BaseDao entity) {
        d.requestContext.currentTransaction().write(entity);
    }

    /**
     * Writes a data object to this transaction which will be commited to the database along with the whole transaction
     */
    public void write(BaseDao entity, boolean validate) {
        d.requestContext.currentTransaction().write(entity, validate);
    }

    /**
     * Writes a public key to the current transaction and hence eventually to the database
     */
    public void write(IPartitionKey partitionKey, MessagePublicKeyDto key) {
        d.requestContext.currentTransaction().write(partitionKey, key);
    }

    /**
     * Deletes an object when the transaction is flushed
     */
    public void delete(BaseDao entity) {
        d.requestContext.currentTransaction().delete(entity);
    }

    public void delete(PUUID id) {
        d.requestContext.currentTransaction().delete(id);
    }

    public void delete(IPartitionKey partitionKey, UUID id) {
        d.requestContext.currentTransaction().delete(PUUID.from(partitionKey, id));
    }

    @SuppressWarnings("unchecked")
    public <T extends BaseDao> T clone(T orig) {
        T cloned = (T) d.merger.cloneObject(orig);
        BaseDaoInternal.setPartitionKey(cloned, BaseDaoInternal.getPartitionKey(orig));
        BaseDaoInternal.setPreviousVersion(cloned, BaseDaoInternal.getPreviousVersion(orig));
        BaseDaoInternal.setMergesVersions(cloned, BaseDaoInternal.getMergesVersions(orig));
        return cloned;
    }

    public <T extends BaseDao> T cloneAndCache(T orig) {
        T cloned = clone(orig);
        currentTransaction().cache(orig.partitionKey(), cloned);
        return cloned;
    }
}
