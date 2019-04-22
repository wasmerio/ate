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

import com.tokera.ate.test.dao.MyAccount;
import com.tokera.ate.io.merge.DataMerger;
import junit.framework.Assert;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;

import java.util.ArrayList;
import java.util.List;
import java.util.UUID;

/**
 *
 * @author John
 */
@TestInstance(TestInstance.Lifecycle.PER_METHOD)
public class MergeLists {

    private DataMerger merger = new DataMerger();

    UUID commonUUID = UUID.randomUUID();
    UUID leftUUID = UUID.randomUUID();
    UUID rightUUID = UUID.randomUUID();

    MyAccount common = new MyAccount();
    MyAccount left = new MyAccount();
    MyAccount right = new MyAccount();

    @Test
    public void testEmpty3way() {
        MyAccount result = (MyAccount) merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 0);
    }

    @Test
    public void testEmpty2way() {
        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 0);
    }

    @Test
    public void testAddLeft3way() {
        left.things.add(leftUUID);

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 1);
        Assert.assertTrue(result.things.contains(leftUUID));
    }

    @Test
    public void testAddLeft2way() {
        left.things.add(leftUUID);

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 1);
        Assert.assertTrue(result.things.contains(leftUUID));
    }

    @Test
    public void testRight3way() {
        right.things.add(rightUUID);

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 1);
        Assert.assertTrue(result.things.contains(rightUUID));
    }

    @Test
    public void testRight2way() {
        right.things.add(rightUUID);

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 1);
        Assert.assertTrue(result.things.contains(rightUUID));
    }
    
    @Test
    public void testAddLeftAndRight3wayA() {
        left.things.add(leftUUID);
        right.things.add(rightUUID);

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 2);
        Assert.assertTrue(result.things.contains(leftUUID));
        Assert.assertTrue(result.things.contains(rightUUID));
    }

    @Test
    public void testAddLeftAndRight2wayA() {
        left.things.add(leftUUID);
        right.things.add(rightUUID);

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 2);
        Assert.assertTrue(result.things.contains(leftUUID));
        Assert.assertTrue(result.things.contains(rightUUID));
    }

    @Test
    public void testAddLeftAndRight3wayB() {
        List<UUID> common = new ArrayList();
        List<UUID> left = new ArrayList();
        List<UUID> right = new ArrayList();
        left.add(leftUUID);
        right.add(rightUUID);

        List<UUID> result = (List<UUID>)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.size() == 2);
        Assert.assertTrue(result.contains(leftUUID));
        Assert.assertTrue(result.contains(rightUUID));
    }

    @Test
    public void testAddLeftAndRight2wayB() {
        List<UUID> common = new ArrayList();
        List<UUID> left = new ArrayList();
        List<UUID> right = new ArrayList();
        left.add(leftUUID);
        right.add(rightUUID);

        List<UUID> result = (List<UUID>)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.size() == 2);
        Assert.assertTrue(result.contains(leftUUID));
        Assert.assertTrue(result.contains(rightUUID));
    }

    @Test
    public void testSubtractRight3way() {
        common.things.add(leftUUID);
        common.things.add(rightUUID);

        left.things.add(leftUUID);
        left.things.add(rightUUID);

        right.things.add(leftUUID);

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 1);
        Assert.assertTrue(result.things.contains(leftUUID));
    }

    @Test
    public void testSubtractRight2way() {
        common.things.add(leftUUID);
        common.things.add(rightUUID);

        left.things.add(leftUUID);
        left.things.add(rightUUID);

        right.things.add(leftUUID);

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 1);
        Assert.assertTrue(result.things.contains(leftUUID));
    }

    @Test
    public void testSubtractLeft3way() {
        common.things.add(leftUUID);
        common.things.add(rightUUID);

        left.things.add(rightUUID);

        right.things.add(leftUUID);
        right.things.add(rightUUID);

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 1);
        Assert.assertTrue(result.things.contains(rightUUID));
    }

    @Test
    public void testSubtractLeft2way() {
        common.things.add(leftUUID);
        common.things.add(rightUUID);

        left.things.add(rightUUID);

        right.things.add(leftUUID);
        right.things.add(rightUUID);

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 1);
        Assert.assertTrue(result.things.contains(rightUUID));
    }

    @Test
    public void testSubtractLeftAndRight3way() {
        common.things.add(leftUUID);
        common.things.add(rightUUID);

        left.things.add(rightUUID);

        right.things.add(leftUUID);

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 0);
    }

    @Test
    public void testSubtractLeftAndRight2way() {
        common.things.add(leftUUID);
        common.things.add(rightUUID);

        left.things.add(rightUUID);

        right.things.add(leftUUID);

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 0);
    }

    @Test
    public void testAdd10000_3way() {
        for (int n = 0; n < 10000; n++) {
            common.things.add(commonUUID);
        }

        for (int n = 0; n < 10000; n++) {
            left.things.add(leftUUID);
        }

        for (int n = 0; n < 10000; n++) {
            right.things.add(rightUUID);
        }

        MyAccount result = (MyAccount)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 20000);
    }

    @Test
    public void testAdd10000_2way() {
        for (int n = 0; n < 10000; n++) {
            common.things.add(commonUUID);
        }

        for (int n = 0; n < 10000; n++) {
            left.things.add(leftUUID);
        }

        for (int n = 0; n < 10000; n++) {
            right.things.add(rightUUID);
        }

        MyAccount result = (MyAccount)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 20000);
    }

    @Test
    public void testEmptyFromNull32way() {
        MyAccount result = (MyAccount)merger.mergeThreeWay(null, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 0);
    }

    @Test
    public void testEmptyFromNull2way() {
        MyAccount result = (MyAccount)merger.mergeApply(null, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 0);
    }

    @Test
    public void testAddLeftFromNull3way() {
        left.things.add(leftUUID);

        MyAccount result = (MyAccount)merger.mergeThreeWay(null, left, null);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 1);
        Assert.assertTrue(result.things.contains(leftUUID));
    }

    @Test
    public void testAddLeftFromNull2way() {
        left.things.add(leftUUID);

        MyAccount result = (MyAccount)merger.mergeApply(null, left, null);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 1);
        Assert.assertTrue(result.things.contains(leftUUID));
    }

    @Test
    public void testRightFromNull3wayA() {
        right.things.add(rightUUID);

        MyAccount result = (MyAccount)merger.mergeThreeWay(null, null, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 1);
        Assert.assertTrue(result.things.contains(rightUUID));
    }

    @Test
    public void testRightFromNull2wayA() {
        right.things.add(rightUUID);

        MyAccount result = (MyAccount)merger.mergeApply(null, null, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 1);
        Assert.assertTrue(result.things.contains(rightUUID));
    }

    @Test
    public void testRightFromNull3wayB() {
        List<UUID> right = new ArrayList<>();
        right.add(rightUUID);

        List<UUID> result = (List<UUID>)merger.mergeThreeWay(null, null, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.size() == 1);
        Assert.assertTrue(result.contains(rightUUID));
    }

    @Test
    public void testRightFromNull2wayB() {
        List<UUID> right = new ArrayList<>();
        right.add(rightUUID);

        List<UUID> result = (List<UUID>)merger.mergeApply(null, null, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.size() == 1);
        Assert.assertTrue(result.contains(rightUUID));
    }

    @Test
    public void testAddLeftAndRightFromNull3way() {
        left.things.add(leftUUID);
        right.things.add(rightUUID);

        MyAccount result = (MyAccount)merger.mergeThreeWay(null, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 2);
        Assert.assertTrue(result.things.contains(leftUUID));
        Assert.assertTrue(result.things.contains(rightUUID));
    }

    @Test
    public void testAddLeftAndRightFromNull2way() {
        left.things.add(leftUUID);
        right.things.add(rightUUID);

        MyAccount result = (MyAccount)merger.mergeApply(null, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertTrue(result.things.size() == 2);
        Assert.assertTrue(result.things.contains(leftUUID));
        Assert.assertTrue(result.things.contains(rightUUID));
    }
}
