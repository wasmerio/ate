package com.esotericsoftware.yamlbeans2.document;

import java.io.IOException;
import java.util.LinkedList;
import java.util.List;

import com.esotericsoftware.yamlbeans2.YamlConfig.WriteConfig;
import com.esotericsoftware.yamlbeans2.YamlException;
import com.esotericsoftware.yamlbeans2.emitter.Emitter;
import com.esotericsoftware.yamlbeans2.emitter.EmitterException;
import com.esotericsoftware.yamlbeans2.parser.Event;
import com.esotericsoftware.yamlbeans2.parser.SequenceStartEvent;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.checkerframework.framework.qual.DefaultQualifier;

@DefaultQualifier(Nullable.class)
@SuppressWarnings({"argument.type.incompatible", "return.type.incompatible", "dereference.of.nullable", "iterating.over.nullable", "method.invocation.invalid", "override.return.invalid", "unnecessary.equals", "known.nonnull", "flowexpr.parse.error.postcondition", "unboxing.of.nullable", "accessing.nullable", "type.invalid.annotations.on.use", "switching.nullable", "initialization.fields.uninitialized"})
public class YamlSequence extends YamlElement implements YamlDocument {

	List<YamlElement> elements = new LinkedList<YamlElement>();

	public int size() {
		return elements.size();
	}
	
	public void addElement(YamlElement element) {
		elements.add(element);
	}
	
	public void deleteElement(int item) throws YamlException {
		elements.remove(item);
	}
	
	public YamlElement getElement(int item) throws YamlException {
		return elements.get(item);
	}

	@Override
	public String toString() {
		StringBuffer sb = new StringBuffer();
		if(anchor!=null) {
			sb.append('&');
			sb.append(anchor);
			sb.append(' ');
		}
		if(tag!=null) {
			sb.append(" !");
			sb.append(tag);
		}
		if(!elements.isEmpty()) {
			sb.append('[');
			for(YamlElement element : elements) {
				sb.append(element.toString());
				sb.append(',');
			}
			sb.setLength(sb.length() - 1);
			sb.append(']');
		}
		return sb.toString();
	}

	
	@Override
	public void emitEvent(Emitter emitter, WriteConfig config) throws EmitterException, IOException {
		emitter.emit(new SequenceStartEvent(anchor, tag, tag==null, false));
		for (YamlElement element : elements)
			element.emitEvent(emitter, config);
		emitter.emit(Event.SEQUENCE_END);	
	}

	public YamlEntry getEntry(String key) throws YamlException {
		throw new YamlException("Can only get entry on mapping!");
	}
	
	public YamlEntry getEntry(int index) throws YamlException {
		throw new YamlException("Can only get entry on mapping!");
	}

	public boolean deleteEntry(String key) throws YamlException {
		throw new YamlException("Can only delete entry on mapping!");
	}

	public void setEntry(String key, boolean value) throws YamlException {
		throw new YamlException("Can only set entry on mapping!");
	}

	public void setEntry(String key, Number value) throws YamlException {
		throw new YamlException("Can only set entry on mapping!");
	}

	public void setEntry(String key, String value) throws YamlException {
		throw new YamlException("Can only set entry on mapping!");
	}

	public void setEntry(String key, YamlElement value) throws YamlException {
		throw new YamlException("Can only set entry on mapping!");
	}

	public void setElement(int item, boolean value) throws YamlException {
		elements.set(item, new YamlScalar(value));
	}

	public void setElement(int item, Number value) throws YamlException {
		elements.set(item, new YamlScalar(value));
	}

	public void setElement(int item, String value) throws YamlException {
		elements.set(item, new YamlScalar(value));
	}

	public void setElement(int item, YamlElement element) throws YamlException {
		elements.set(item, element);
	}

	public void addElement(boolean value) throws YamlException {
		elements.add(new YamlScalar(value));
	}

	public void addElement(Number value) throws YamlException {
		elements.add(new YamlScalar(value));
	}

	public void addElement(String value) throws YamlException {
		elements.add(new YamlScalar(value));
	}


}
