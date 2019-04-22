/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.common;

import com.google.api.client.util.Strings;
import java.util.AbstractMap.SimpleImmutableEntry;
import java.util.Arrays;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;
import static java.util.stream.Collectors.mapping;
import static java.util.stream.Collectors.toList;

/**
 * URI helper class for splitting URL query parameters
 */
public class UriTools
{
    public static Map<String, List<String>> splitQuery(String  url) {
        if (Strings.isNullOrEmpty(url)) {
            return Collections.emptyMap();
        }
        return Arrays.stream(url.split("&"))
                .map(UriTools::splitQueryParameter)
                .collect(Collectors.groupingBy(SimpleImmutableEntry::getKey, LinkedHashMap::new, mapping(Map.Entry::getValue, toList())));
    }

    protected static SimpleImmutableEntry<String, String> splitQueryParameter(String it) {
        final int idx = it.indexOf("=");
        final String key = idx > 0 ? it.substring(0, idx) : it;
        final String value = idx > 0 && it.length() > idx + 1 ? it.substring(idx + 1) : "";
        return new SimpleImmutableEntry<>(key, value);
    }
}
