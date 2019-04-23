package com.esotericsoftware.yamlbeans2.document;

import java.io.IOException;

import com.esotericsoftware.yamlbeans2.YamlConfig.WriteConfig;
import com.esotericsoftware.yamlbeans2.emitter.Emitter;
import com.esotericsoftware.yamlbeans2.emitter.EmitterException;
import com.esotericsoftware.yamlbeans2.parser.AliasEvent;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.checkerframework.framework.qual.DefaultQualifier;

@DefaultQualifier(Nullable.class)
@SuppressWarnings({"argument.type.incompatible", "return.type.incompatible", "dereference.of.nullable", "iterating.over.nullable", "method.invocation.invalid", "override.return.invalid", "unnecessary.equals", "known.nonnull", "flowexpr.parse.error.postcondition", "unboxing.of.nullable", "accessing.nullable", "type.invalid.annotations.on.use", "switching.nullable", "initialization.fields.uninitialized"})
public class YamlAlias extends YamlElement {

	@Override
	public void emitEvent(Emitter emitter, WriteConfig config) throws EmitterException, IOException {
		emitter.emit(new AliasEvent(anchor));
	}
	
	@Override
	public String toString() {
		return "*" + anchor;
	}
}
