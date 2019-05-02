package com.tokera.ate.test.dto;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.units.EmailAddress;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import javax.validation.constraints.Pattern;
import javax.validation.constraints.Size;

@Dependent
@YamlTag("dto.new.account")
public class NewAccountDto {

    @JsonProperty
    @Nullable
    @Size(min=1, max=512)
    @Pattern(regexp="[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?", message="Invalid email")//if the field contains email address consider using this annotation to enforce field validation
    private @EmailAddress String email;
    @JsonProperty
    @Nullable
    private String description;

    public @Nullable String getEmail() {
        return email;
    }

    public void setEmail(String email) {
        this.email = email;
    }

    public @Nullable String getDescription() {
        return description;
    }

    public void setDescription(String description) {
        this.description = description;
    }
}
