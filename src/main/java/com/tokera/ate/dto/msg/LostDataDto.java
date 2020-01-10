package com.tokera.ate.dto.msg;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.YamlTag;

import javax.enterprise.context.Dependent;
import java.util.ArrayList;
import java.util.List;

@YamlTag("lost.data")
@Dependent
public class LostDataDto
{
    @JsonProperty
    public MessageDataDto data;
    @JsonProperty
    public MessageMetaDto meta;
    @JsonProperty
    public ArrayList<String> reasons;
}
