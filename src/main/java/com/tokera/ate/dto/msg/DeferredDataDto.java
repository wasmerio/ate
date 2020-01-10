package com.tokera.ate.dto.msg;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.YamlTag;

import javax.enterprise.context.Dependent;
import java.util.ArrayList;
import java.util.Date;

@YamlTag("deferred.data")
@Dependent
public class DeferredDataDto
{
    @JsonProperty
    public MessageDataMetaDto msg;
    @JsonProperty
    public Date deferStart = new Date();
    @JsonProperty
    public int deferCount = 1;
    @JsonProperty
    public ArrayList<String> reasons;
}
