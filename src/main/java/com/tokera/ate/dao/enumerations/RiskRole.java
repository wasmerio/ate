package com.tokera.ate.dao.enumerations;

import com.tokera.ate.annotations.YamlTag;


/**
 * Allows a level of risk to be associated with particular method calls and the access risks provided by a token
 */
@YamlTag("enum.risk.role")
public enum RiskRole {

    NONE("No authority to perform any actions"),
    LOW("Represents the authority to carry out low risk transactions such as balance enquires"),
    MEDIUM("Represents the authority to carry out medium risk transactions such as starting and stopping virtual machines"),
    HIGH("Represents the authority to carry out high risk transactions such as transferring money");

    private final String description;

    RiskRole(String description) {
        this.description = description;
    }

    public String getDescription() {
        return description;
    }
    
    public static RiskRole getEnumByCode(String code) {
        for (RiskRole thisEnum : RiskRole.values()) {
            if (thisEnum.name().equalsIgnoreCase(code)) {
                return thisEnum;
            }
        }
        throw new RuntimeException("Unable to parse risk role [" + code + "]");
    }
}
