package com.tokera.ate.test.weld;

import com.tokera.ate.KafkaServer;
import com.tokera.ate.ZooServer;
import com.tokera.ate.test.dao.MyAccount;
import com.tokera.ate.test.dao.MyThing;
import org.jboss.weld.bootstrap.spi.BeanDiscoveryMode;
import org.jboss.weld.environment.se.Weld;
import org.jboss.weld.junit5.WeldInitiator;
import org.jboss.weld.junit5.WeldJunit5Extension;
import org.jboss.weld.junit5.WeldSetup;
import org.junit.jupiter.api.*;
import org.junit.jupiter.api.extension.ExtendWith;

import javax.enterprise.context.RequestScoped;
import javax.enterprise.inject.spi.CDI;

@TestInstance(TestInstance.Lifecycle.PER_CLASS)
@ExtendWith(WeldJunit5Extension.class)
public class WeldTests {

    @SuppressWarnings("argument.type.incompatible")
    @WeldSetup
    public WeldInitiator weld = WeldInitiator
            .from(new Weld()
                    .setBeanDiscoveryMode(BeanDiscoveryMode.ANNOTATED)
                    .enableDiscovery()
                    .addBeanClass(MyAccount.class)
                    .addBeanClass(MyThing.class))
            .activate(RequestScoped.class)
            .build();

    //@Test
    public void zookeeper() {
        CDI.current().select(ZooServer.class).get().touch();
    }

    //@Test
    public void kafka() {
        CDI.current().select(KafkaServer.class).get().touch();
    }
}
