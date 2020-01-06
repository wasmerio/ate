package com.tokera.ate.scopes;

import javax.enterprise.context.spi.Context;
import java.lang.annotation.Annotation;

public interface IScopeContext extends Context {

    IScope getLocal();

    IScope getLocalWithInactive();

    void setLocal(IScope scope);
}
