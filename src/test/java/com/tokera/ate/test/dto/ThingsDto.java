package com.tokera.ate.test.dto;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.test.dao.MyThing;
import com.tokera.ate.units.EmailAddress;
import com.tokera.ate.units.Secret;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;
import java.util.List;

@Dependent
@YamlTag("dto.things.list")
public class ThingsDto {

    @JsonProperty
    public List<MyThing> things;
}
