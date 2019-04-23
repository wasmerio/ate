package com.tokera.ate.dao.enumerations;

import com.tokera.ate.annotations.YamlTag;

/**
 * Represents the type of user that is allows to interact with API methods and the type of user that authenticated
 * to generate a login token
 */
@YamlTag("enum.user.role")
public enum UserRole {

    ANYTHING("This could be any type of user, from a human being to a automated batch system."),
    HUMAN("Represents a human being that interacts with the systems via interfaces."),
    NPA("Represents a non-personal account (i.e. a robot).");
    
    private final String description;

    UserRole(String description) {
        this.description = description;
    }

    public String getDescription() {
        return description;
    }
}
