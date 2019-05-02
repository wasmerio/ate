package com.tokera.ate.test.chain;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.extensions.SerializableObjectsExtension;
import com.tokera.ate.io.repo.IObjectSerializer;
import com.tokera.ate.test.dao.MyAccount;
import com.tokera.ate.test.dao.MyThing;
import org.jboss.weld.bootstrap.spi.BeanDiscoveryMode;
import org.jboss.weld.environment.se.Weld;
import org.jboss.weld.junit5.WeldInitiator;
import org.jboss.weld.junit5.WeldJunit5Extension;
import org.jboss.weld.junit5.WeldSetup;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;
import org.junit.jupiter.api.extension.ExtendWith;

import javax.enterprise.context.RequestScoped;
import javax.enterprise.inject.spi.CDI;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.Map;
import java.util.UUID;

@TestInstance(TestInstance.Lifecycle.PER_METHOD)
@ExtendWith(WeldJunit5Extension.class)
public class SerializerTests {

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

    @Test
    public void testSerializeAndDeserialize() {

        MyAccount left = new MyAccount();
        left.textFiles.put("blah", UUID.randomUUID());
        left.isPublic = true;
        left.d1 = 1.0;
        left.f1 = 1.0f;
        left.num1 = BigInteger.TEN;
        left.num2 = BigDecimal.TEN;

        IObjectSerializer serializer = CDI.current().select(IObjectSerializer.class).get();
        byte[] data = serializer.serializeObj(left);

        Class<BaseDao> clazz = CDI.current().select(SerializableObjectsExtension.class).get().findClass(MyAccount.class.getName(), BaseDao.class);
        MyAccount right = (MyAccount)serializer.deserializeObj(data, clazz);

        Assertions.assertEquals(left.isPublic, right.isPublic);
        Assertions.assertEquals(left.textFiles.size(), right.textFiles.size());
        Assertions.assertEquals(left.f1, right.f1);
        Assertions.assertEquals(left.d1, right.d1);
        Assertions.assertEquals(left.num1, right.num1);
        Assertions.assertEquals(left.num2, right.num2);
        for (Map.Entry<String, UUID> pair : left.textFiles.entrySet()) {
            UUID val = right.textFiles.getOrDefault(pair.getKey(), UUID.randomUUID());
            Assertions.assertEquals(pair.getValue(), val);
        }
    }
}
