/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.providers;

import com.esotericsoftware.yamlbeans2.YamlException;
import com.esotericsoftware.yamlbeans2.scalar.ScalarSerializer;
import com.tokera.ate.common.StringTools;
import org.checkerframework.checker.nullness.qual.Nullable;

public class LongSerializer implements ScalarSerializer<Long>
{
    public LongSerializer() {
        
    }
    
    @Override
    public @Nullable String write(@Nullable Long t) throws YamlException {
        if (t == null) return "null";
        return Long.toString(t);
    }

    @Override
    public @Nullable Long read(@Nullable String _val) throws YamlException {
        String val = StringTools.makeOneLineOrNull(_val);
        val = StringTools.specialParse(val);
        if (val == null || val.length() <= 0) return null;

        return Long.parseLong(val);
    }
}
