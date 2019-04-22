package com.tokera.ate.events;

import com.tokera.ate.common.StringTools;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.units.DomainName;
import com.tokera.ate.units.EmailAddress;

/**
 * Event thats triggered whenever the Token scope is entered
 */
public class TokenScopeChangedEvent {

    private @DomainName String domain;

    public TokenScopeChangedEvent(TokenDto token) {
        @EmailAddress String email = token.getUsername();
        this.domain = StringTools.getDomain(email);
    }

    public TokenScopeChangedEvent(@DomainName String domain) {
        this.domain = domain;
    }

    public @DomainName String getDomain() {
        return domain;
    }

    public void setDomain(@DomainName String domain) {
        this.domain = domain;
    }
}
