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
public class YamlScalar extends YamlElement {

	String value;
	
	public YamlScalar() {
	}
	
	public YamlScalar(Object value) {
		this.value = String.valueOf(value);
	}

	public String getValue() {
		return value;
	}
	
	public void setValue(String value) {
		this.value = value;
	}

	@Override
	public String toString() {
		StringBuilder sb = new StringBuilder();
		if(anchor!=null) {
			sb.append('&');
			sb.append(anchor);
			sb.append(' ');
		}
		sb.append(value);
		if(tag!=null) {
			sb.append(" !");
			sb.append(tag);
		}
		return sb.toString();
	}
	
	@Override
	public void emitEvent(Emitter emitter, WriteConfig config) throws EmitterException, IOException {
		char style = 0; // TODO determine style
		emitter.emit(new ScalarEvent(anchor, tag, new boolean[] {true, true}, value, style));
	}
}
