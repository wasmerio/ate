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
public class MergeBoolean {

    private DataMerger merger = new DataMerger();

    MyAccount common = new MyAccount();
    MyAccount left = new MyAccount();
    MyAccount right = new MyAccount();

    @Test
    public void testNoChangeFalse3way() {
        common.isPublic = false;
        left.isPublic = false;
        right.isPublic = false;

        MyAccount result = (MyAccount) merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertEquals(false, result.isPublic);
    }

    @Test
    public void testNoChangeFalse2way() {
        common.isPublic = false;
        left.isPublic = false;
        right.isPublic = false;

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertEquals(false, result.isPublic);
    }

    @Test
    public void testNoChangeTrue3way() {
        common.isPublic = true;
        left.isPublic = true;
        right.isPublic = true;

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertEquals(true, result.isPublic);
    }

    @Test
    public void testNoChangeTrue2way() {
        common.isPublic = true;
        left.isPublic = true;
        right.isPublic = true;

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertEquals(true, result.isPublic);
    }

    @Test
    public void testRightTrue3way() {
        common.isPublic = false;
        left.isPublic = false;
        right.isPublic = true;

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertEquals(true, result.isPublic);
    }

    @Test
    public void testRightTrue2way() {
        common.isPublic = false;
        left.isPublic = false;
        right.isPublic = true;

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertEquals(true, result.isPublic);
    }

    @Test
    public void testRightFalse3way() {
        common.isPublic = true;
        left.isPublic = true;
        right.isPublic = false;

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertEquals(false, result.isPublic);
    }

    @Test
    public void testRightFalse2way() {
        common.isPublic = true;
        left.isPublic = true;
        right.isPublic = false;

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertEquals(false, result.isPublic);
    }

    @Test
    public void testLeftTrue3way() {
        common.isPublic = false;
        left.isPublic = true;
        right.isPublic = false;

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertEquals(true, result.isPublic);
    }

    @Test
    public void testLeftTrue2way() {
        common.isPublic = false;
        left.isPublic = true;
        right.isPublic = false;

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertEquals(true, result.isPublic);
    }

    @Test
    public void testLeftFalse3way() {
        common.isPublic = true;
        left.isPublic = false;
        right.isPublic = true;

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertEquals(false, result.isPublic);
    }

    @Test
    public void testLeftFalse2way() {
        common.isPublic = true;
        left.isPublic = false;
        right.isPublic = true;

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertEquals(false, result.isPublic);
    }

    @Test
    public void testLeftAndRightTrue3way() {
        common.isPublic = false;
        left.isPublic = true;
        right.isPublic = true;

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertEquals(true, result.isPublic);
    }

    @Test
    public void testLeftAndRightTrue2way() {
        common.isPublic = false;
        left.isPublic = true;
        right.isPublic = true;

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertEquals(true, result.isPublic);
    }

    @Test
    public void testLeftAndRightFalse3way() {
        common.isPublic = true;
        left.isPublic = false;
        right.isPublic = false;

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertEquals(false, result.isPublic);
    }

    @Test
    public void testLeftAndRightFalse2way() {
        common.isPublic = true;
        left.isPublic = false;
        right.isPublic = false;

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertEquals(false, result.isPublic);
    }
}
