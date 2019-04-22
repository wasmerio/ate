/*
 * Copyright 2018 John.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
package com.tokera.ate.test.merge;

import com.tokera.ate.io.merge.DataMerger;
import com.tokera.ate.test.dao.MyAccount;
import junit.framework.Assert;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;

/**
 *
 * @author John
 */
@TestInstance(TestInstance.Lifecycle.PER_METHOD)
public class MergeStrings {

    private DataMerger merger = new DataMerger();

    MyAccount common = new MyAccount();
    MyAccount left = new MyAccount();
    MyAccount right = new MyAccount();

    @Test
    public void testNoChange3way() {
        common.description = "www.test.com";
        left.description = "www.test.com";
        right.description = "www.test.com";

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(desc);
        Assert.assertEquals("www.test.com", desc);
    }

    @Test
    public void testNoChange2way() {
        common.description = "www.test.com";
        left.description = "www.test.com";
        right.description = "www.test.com";

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(desc);
        Assert.assertEquals("www.test.com", desc);
    }
    
    @Test
    public void testRightBias3way() {
        common.description = "test base";
        left.description = "test left";
        right.description = "www.test.com";

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(desc);
        Assert.assertEquals("www.test.com", desc);
    }

    @Test
    public void testRightBias2way() {
        common.description = "test base";
        left.description = "test left";
        right.description = "www.test.com";

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(desc);
        Assert.assertEquals("test left", desc);
    }

    @Test
    public void testRight3way() {
        common.description = "test base";
        left.description = "test base";
        right.description = "www.test.com";

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(desc);
        Assert.assertEquals("www.test.com", desc);
    }

    @Test
    public void testRight2way() {
        common.description = "test base";
        left.description = "test base";
        right.description = "www.test.com";

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(desc);
        Assert.assertEquals("www.test.com", desc);
    }

    @Test
    public void testLeft3way() {
        common.description = "test base";
        left.description = "test left";
        right.description = "test base";

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(desc);
        Assert.assertEquals("test left", desc);
    }

    @Test
    public void testLeft2way() {
        common.description = "test base";
        left.description = "test left";
        right.description = "test base";

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(desc);
        Assert.assertEquals("test left", desc);
    }

    @Test
    public void testRightNull3way() {
        common.description = "test base";
        left.description = "test base";
        right.description = null;

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc == null : "@AssumeAssertion(nullness): Must be null";
        Assert.assertTrue(desc == null);
    }

    @Test
    public void testRightNull2way() {
        common.description = "test base";
        left.description = "test base";
        right.description = null;

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc == null : "@AssumeAssertion(nullness): Must be null";
        Assert.assertTrue(desc == null);
    }

    @Test
    public void testRightBiasOverNull3way() {
        common.description = "test base";
        left.description =  null;
        right.description = "www.test.com";

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(desc);
        Assert.assertEquals("www.test.com", desc);
    }

    @Test
    public void testRightBiasOverNull2way() {
        common.description = "test base";
        left.description =  null;
        right.description = "www.test.com";

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc == null;
    }

    @Test
    public void testDoubleNull3way() {
        common.description = "test base";
        left.description = null;
        right.description = null;

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc == null : "@AssumeAssertion(nullness): Must be null";
        Assert.assertTrue(desc == null);
    }

    @Test
    public void testDoubleNull2way() {
        common.description = "test base";
        left.description = null;
        right.description = null;

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc == null : "@AssumeAssertion(nullness): Must be null";
        Assert.assertTrue(desc == null);
    }

    @Test
    public void testLeftOverNullA_3way() {
        common.description = null;
        left.description = "test left";
        right.description = null;

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(desc);
        Assert.assertEquals("test left", desc);
    }

    @Test
    public void testLeftOverNullA_2way() {
        common.description = null;
        left.description = "test left";
        right.description = null;

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(desc);
        Assert.assertEquals("test left", desc);
    }

    @Test
    public void testLeftOverNullB_3way() {
        common.description = null;
        left.description = "test left";
        right.description = "test right";

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(desc);
        Assert.assertEquals("test right", desc);
    }

    @Test
    public void testLeftOverNullB_2way() {
        common.description = null;
        left.description = "test left";
        right.description = "test right";

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(desc);
        Assert.assertEquals("test right", desc);
    }

    @Test
    public void testRightOverNull3way() {
        common.description = null;
        left.description = null;
        right.description = "test right";

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(desc);
        Assert.assertEquals("test right", desc);
    }

    @Test
    public void testRightOverNull2way() {
        common.description = null;
        left.description = null;
        right.description = "test right";

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        String desc = result.description;
        assert desc != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(desc);
        Assert.assertEquals("test right", desc);
    }
}
