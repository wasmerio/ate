package com.tokera.ate.enumerations;

import com.tokera.ate.annotations.YamlTag;

import javax.enterprise.context.Dependent;

@Dependent
@YamlTag("enum.symbolic.link.type")
public enum SymbolicLinkType {
    File,
    Folder
}
