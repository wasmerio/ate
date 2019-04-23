package com.esotericsoftware.yamlbeans2.document;

import java.io.IOException;

import com.esotericsoftware.yamlbeans2.YamlConfig.WriteConfig;
import com.esotericsoftware.yamlbeans2.emitter.Emitter;
import com.esotericsoftware.yamlbeans2.emitter.EmitterException;
import com.esotericsoftware.yamlbeans2.parser.ScalarEvent;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.checkerframework.framework.qual.DefaultQualifier;

@DefaultQualifier(Nullable.class)
@SuppressWarnings({"argument.type.incompatible", "return.type.incompatible", "dereference.of.nullable", "iterating.over.nullable", "method.invocation.invalid", "override.return.invalid", "unnecessary.equals", "known.nonnull", "flowexpr.parse.error.postcondition", "unboxing.of.nullable", "accessing.nullable", "type.invalid.annotations.on.use", "switching.nullable", "initialization.fields.uninitialized"})
public class YamlEntry {
	
	YamlScalar key;
	YamlElement value;
	
	public YamlEntry(YamlScalar key, YamlElement value) {
		this.key = key;
		this.value = value;
	}

	@Override
	public String toString() {
		StringBuilder sb = new StringBuilder();
		sb.append(key.toString());
		sb.append(':');
		sb.append(value.toString());
		return sb.toString();
	}
	
	public YamlScalar getKey() {
		return key;
	}
	
	public YamlElement getValue() {
		return value;
	}
	
	public void setKey(YamlScalar key) {
		this.key = key;
	}
	
	public void setValue(YamlElement value) {
		this.value = value;
	}

	public void setValue(boolean value) {
		this.value = new YamlScalar(value);
	}
	
	public void setValue(Number value) {
		this.value = new YamlScalar(value);
	}
	
	public void setValue(String value) {
		this.value = new YamlScalar(value);
	}

	public void emitEvent(Emitter emitter, WriteConfig config) throws EmitterException, IOException {
		key.emitEvent(emitter, config);
		if(value==null)
			emitter.emit(new ScalarEvent(null, null, new boolean[] {true, true}, null, (char)0));
		else
			value.emitEvent(emitter, config);
	}
	
}
