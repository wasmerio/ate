/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.providers;

import com.esotericsoftware.yamlbeans2.YamlException;
import com.esotericsoftware.yamlbeans2.scalar.ScalarSerializer;
import com.tokera.ate.common.StringTools;
import com.tokera.ate.common.UUIDTools;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.UUID;

public class UuidSerializer implements ScalarSerializer<UUID>
{
    public UuidSerializer() {
        
    }
    
    @Override
    public @Nullable String write(@Nullable UUID t) throws YamlException {
        if (t == null) return "null";
        return t.toString();
    }

    @Override
    public @Nullable UUID read(@Nullable String _val) throws YamlException {
        String val = StringTools.makeOneLineOrNull(_val);
        val = StringTools.specialParse(val);
        if (val == null || val.length() <= 0) return null;

        return UUIDTools.parseUUIDorNull(val);
    }
}
