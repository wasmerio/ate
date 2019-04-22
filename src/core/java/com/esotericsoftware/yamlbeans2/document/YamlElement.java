package com.esotericsoftware.yamlbeans2.document;

import java.io.IOException;

import com.esotericsoftware.yamlbeans2.YamlConfig.WriteConfig;
import com.esotericsoftware.yamlbeans2.emitter.Emitter;
import com.esotericsoftware.yamlbeans2.emitter.EmitterException;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.checkerframework.framework.qual.DefaultQualifier;

@DefaultQualifier(Nullable.class)
@SuppressWarnings({"argument.type.incompatible", "return.type.incompatible", "dereference.of.nullable", "iterating.over.nullable", "method.invocation.invalid", "override.return.invalid", "unnecessary.equals", "known.nonnull", "flowexpr.parse.error.postcondition", "unboxing.of.nullable", "accessing.nullable", "type.invalid.annotations.on.use", "switching.nullable", "initialization.fields.uninitialized"})
public abstract class YamlElement {

	String tag;
	String anchor;
	
	public void setTag(String tag) {
		this.tag = tag;
	}
	
	public void setAnchor(String anchor) {
		this.anchor = anchor;
	}
	
	public String getTag() {
		return tag;
	}
	
	public String getAnchor() {
		return anchor;
	}

	public abstract void emitEvent(Emitter emitter, WriteConfig config) throws EmitterException, IOException;
}
