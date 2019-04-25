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

import java.text.ParseException;
import java.text.SimpleDateFormat;
import java.util.Date;

public class DateSerializer implements ScalarSerializer<Date>
{
    SimpleDateFormat format = new SimpleDateFormat("yyyy-MM-dd HH:mm:ss.SSSZ");
    
    public DateSerializer() {
        
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable String write(@Nullable Date t) throws YamlException {
        if (t == null) return "null";
        return format.format(t);
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable Date read(@Nullable String _val) throws YamlException {
        String val = StringTools.makeOneLineOrNull(_val);
        val = StringTools.specialParse(val);
        if (val == null || val.length() <= 0) return null;

        try {
            return format.parse(val);
        } catch (ParseException | NumberFormatException ex) {
            return null;
        }
    }
}
