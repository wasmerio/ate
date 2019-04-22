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
package org.tomitribe.microscoped.core;

import org.checkerframework.checker.nullness.qual.Nullable;
import org.checkerframework.framework.qual.DefaultQualifier;

import javax.enterprise.context.spi.Contextual;
import javax.enterprise.context.spi.CreationalContext;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

@DefaultQualifier(Nullable.class)
@SuppressWarnings({"argument.type.incompatible", "return.type.incompatible", "dereference.of.nullable", "iterating.over.nullable", "method.invocation.invalid", "override.return.invalid", "unnecessary.equals", "known.nonnull", "flowexpr.parse.error.postcondition", "unboxing.of.nullable", "accessing.nullable", "type.invalid.annotations.on.use", "switching.nullable", "initialization.fields.uninitialized"})
class Scope<Key> {

    private final Instance<?> NOTHING = new Instance<>(null, null, null);

    private final Map<Contextual<?>, Instance> instances = new ConcurrentHashMap<>();

    private final Key key;

    public Scope(final Key key) {
        this.key = key;
    }

    public Key getKey() {
        return key;
    }

    /**
     * Returns an instance of the Bean (Contextual), creating it if necessary
     *
     * @param contextual the Bean type to create
     * @param <T> the Java type of the bean instance itself
     * @return existing or newly created bean instance, never null
     */
    public <T> T get(final Contextual<T> contextual, final CreationalContext<T> creationalContext) {
        return (T) instances.computeIfAbsent(contextual, c -> new Instance<>(contextual, creationalContext)).get();
    }

    /**
     * Returns the existing instance of the Bean or null if none exists yet
     *
     * @param contextual the Bean type to create
     * @param <T> the Java type of the bean instance itself
     * @return existing the bean instance or null
     */
    public <T> T get(final Contextual<T> contextual) {
        return (T) instances.getOrDefault(contextual, NOTHING).get();
    }

    /**
     * Destroy all the instances in this scope
     */
    public void destroy() {
        // TODO We really should ensure no more instances can be added during or after this
        instances.values().stream().forEach(Instance::destroy);
        instances.clear();
    }

    private class Instance<T> {

        private final T instance;
        private final CreationalContext<T> creationalContext;
        private final Contextual<T> contextual;

        public Instance(final Contextual<T> contextual, final CreationalContext<T> creationalContext) {

            this(contextual, creationalContext, contextual.create(creationalContext));
        }

        public Instance(Contextual<T> contextual, CreationalContext<T> creationalContext, T instance) {
            this.instance = instance;
            this.creationalContext = creationalContext;
            this.contextual = contextual;
        }

        public T get() {
            return instance;
        }

        public void destroy() {
            contextual.destroy(instance, creationalContext);
        }
    }
}
