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

import java.text.SimpleDateFormat;

public class BooleanSerializer implements ScalarSerializer<Boolean>
{
    SimpleDateFormat format = new SimpleDateFormat("yyyy-MM-dd HH:mm:ss.SSSZ");
    
    public BooleanSerializer() {
        
    }
    
    @Override
    public @Nullable String write(@Nullable Boolean t) throws YamlException {
        if (t == null) return "null";
        if (t == true) return "true";
        return "false";
    }

    @Override
    public @Nullable Boolean read(@Nullable String _val) throws YamlException {
        String val = StringTools.makeOneLineOrNull(_val);
        val = StringTools.specialParse(val);
        if (val == null || val.length() <= 0) return null;

        if ("1".equalsIgnoreCase(val)) return true;
        if ("true".equalsIgnoreCase(val)) return true;
        if ("yes".equalsIgnoreCase(val)) return true;
        if ("on".equalsIgnoreCase(val)) return true;
        
        if ("0".equalsIgnoreCase(val)) return false;
        if ("false".equalsIgnoreCase(val)) return false;
        if ("no".equalsIgnoreCase(val)) return false;
        if ("off".equalsIgnoreCase(val)) return false;
        
        return null;
    }
}
