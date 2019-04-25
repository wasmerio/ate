/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.providers;

import com.esotericsoftware.yamlbeans.YamlException;
import com.esotericsoftware.yamlbeans.scalar.ScalarSerializer;
import com.tokera.ate.common.StringTools;
import org.checkerframework.checker.nullness.qual.Nullable;

public class IntegerSerializer implements ScalarSerializer<Integer>
{
    public IntegerSerializer() {
        
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable String write(@Nullable Integer t) throws YamlException {
        if (t == null) return "null";
        return Integer.toString(t);
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable Integer read(@Nullable String _val) throws YamlException {
        String val = StringTools.makeOneLineOrNull(_val);
        val = StringTools.specialParse(val);
        if (val == null || val.length() <= 0) return null;

        return Integer.parseInt(val);
    }
}
