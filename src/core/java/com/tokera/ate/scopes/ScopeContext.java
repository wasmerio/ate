/*
 * Licensed to the Apache Software Foundation (ASF) under one or more
 * contributor license agreements.  See the NOTICE file distributed with
 * this work for additional information regarding copyright ownership.
 * The ASF licenses this file to You under the Apache License, Version 2.0
 * (the "License"); you may not use this file except in compliance with
 * the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */
package com.tokera.ate.scopes;

import com.google.common.cache.Cache;
import com.google.common.cache.CacheBuilder;

import javax.enterprise.context.spi.Context;
import javax.enterprise.context.spi.Contextual;
import javax.enterprise.context.spi.CreationalContext;
import java.lang.annotation.Annotation;
import java.util.Map;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;

@SuppressWarnings({"argument.type.incompatible", "return.type.incompatible", "dereference.of.nullable", "iterating.over.nullable", "method.invocation.invalid", "override.return.invalid", "unnecessary.equals", "known.nonnull", "flowexpr.parse.error.postcondition", "unboxing.of.nullable", "accessing.nullable", "type.invalid.annotations.on.use", "switching.nullable", "initialization.fields.uninitialized", "bound.type.incompatible", "methodref.receiver.bound.invalid", "override.receiver.invalid", "override.param.invalid"})
public class ScopeContext<Key> implements Context {

    private final Cache<Key, Scope<Key>> scopes;

    /**
     * The use of an AtomicReference inside the ThreadLocal is a clever way to
     * be able to do several operations against the "slot" for this thread
     * without repeat lookups to the ThreadLocal's underlying map.
     *
     * We use an AtomicReference for the "slot" simply because it has several
     * convenience methods for inplace editing such as getAndSet.
     *
     * Similar to if Map.Entry had a 'setValue(Object)' method
     */
    private final ThreadLocal<AtomicReference<Scope<Key>>> active = ThreadLocal.withInitial(this::inactive);

    private final Class<? extends Annotation> scopeAnnotation;

    public ScopeContext(final Class<? extends Annotation> scopeAnnotation, long expiresAfter) {
        this.scopeAnnotation = scopeAnnotation;

        CacheBuilder<Object, Object> builder = CacheBuilder
                .newBuilder()
                .concurrencyLevel(32);
        if (expiresAfter > 0) {
            builder = builder
                    .expireAfterAccess(expiresAfter, TimeUnit.MINUTES);
        }
        this.scopes = builder
                .build();

    }

    /**
     * Activate the new Scope and return the old scope for future reassociation
     *
     * If no scope instance existed associated with this key, a new scope will
     * be lazily created
     *
     * @param key the key to serve as the "center of the world" of the scope
     * @return returns the previous "center of the world" so it can be
     * reassociated when this scope exits
     */
    public Key enter(Key key) {

        final Scope<Key> scope;
        try {
            scope = scopes.get(key, () -> new Scope<>(key));
            return scope().getAndSet(scope).getKey();
        } catch (ExecutionException ex) {
            return null;
        }
    }

    /**
     * Exits the currentRights scope and connects the thread with the previously
     * associated scope
     *
     * @param previous the key of the previously active scope or null
     */
    @SuppressWarnings({"unchecked method invocation", "unchecked_conversion"})
    public void exit(Key previous) {
        final AtomicReference<Scope<Key>> reference = scope();

        if (previous == null) {

            reference.set(inactiveScope);

        } else {

            // Can possibly be null of Scope was destroyed
            final Scope<Key> scope = scopes.getIfPresent(previous);
            if (scope != null) {
                reference.set(scope);
            }
        }
    }

    /**
     * Destroys the scope and all instance in that scope
     *
     * @param key the key of the scope
     */
    public void destroy(Key key) {
        Scope scope = scopes.getIfPresent(key);
        if (scope != null) {
            scope.destroy();
        }
        scopes.invalidate(key);
    }

    public void destroyAll() {
        Map<Key, Scope<Key>> scopesMap = scopes.asMap();
        scopesMap.values().stream().forEach(Scope::destroy);
        scopes.invalidateAll();
    }

    @Override
    public <T> T get(Contextual<T> contextual, CreationalContext<T> creationalContext) {
        return scope().get().get(contextual, creationalContext);
    }

    @Override
    public <T> T get(Contextual<T> contextual) {
        return scope().get().get(contextual);
    }

    private AtomicReference<Scope<Key>> scope() {
        return active.get();
    }

    @Override
    public boolean isActive() {
        return scope() != null;
    }

    @Override
    public Class<? extends Annotation> getScope() {
        return scopeAnnotation;
    }

    private final Scope<Key> inactiveScope = new Scope<Key>(null) {
        @Override
        public <T> T get(Contextual<T> contextual, CreationalContext<T> creationalContext) {
            throw new IllegalStateException("Scope Not Active");
        }

        @Override
        public <T> T get(Contextual<T> contextual) {
            throw new IllegalStateException("Scope Not Active");
        }

        @Override
        public void destroy() {
            throw new IllegalStateException("Scope Not Active");
        }
    };

    private AtomicReference<Scope<Key>> inactive() {
        return new AtomicReference<>(inactiveScope);
    }
}
