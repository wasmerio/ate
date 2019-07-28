package com.tokera.ate.dto;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;

@Dependent
@YamlTag("roles.pair")
public class RolesPairDto
{
    @Nullable
    @JsonProperty
    public MessagePublicKeyDto read;
    @Nullable
    @JsonProperty
    public MessagePublicKeyDto write;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public RolesPairDto() {
    };

    public RolesPairDto(@Nullable MessagePublicKeyDto read, @Nullable MessagePublicKeyDto write)
    {
        this.read = read;
        this.write = write;
    }
}
