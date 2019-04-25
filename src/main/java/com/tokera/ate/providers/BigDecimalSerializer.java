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

import java.math.BigDecimal;

public class BigDecimalSerializer implements ScalarSerializer<BigDecimal>
{
    public BigDecimalSerializer() {
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable String write(@Nullable BigDecimal t) throws @Nullable YamlException {
        if (t == null) return "null";
        return t.toPlainString();
    }

    @SuppressWarnings("override.return.invalid")
    @Override
    public @Nullable BigDecimal read(@Nullable String _val) throws @Nullable YamlException {
        String val = StringTools.makeOneLineOrNull(_val);
        val = StringTools.specialParse(val);
        if (val == null || val.length() <= 0) return null;

        return new BigDecimal(val);
    }
}
