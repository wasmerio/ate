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

import com.tokera.ate.dto.msg.MessageDataHeaderDto;
import com.tokera.ate.io.merge.DataMerger;
import com.tokera.ate.test.dao.MyAccount;
import junit.framework.Assert;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;

import java.util.HashSet;
import java.util.Set;
import java.util.UUID;

/**
 *
 * @author John
 */
@TestInstance(TestInstance.Lifecycle.PER_METHOD)
public class MergeSets {

    private DataMerger merger = new DataMerger();

    String commonUUID = "the";
    String leftUUID = "cat";
    String rightUUID = "ran";

    MessageDataHeaderDto common;
    MessageDataHeaderDto left;
    MessageDataHeaderDto right;

    public MergeSets() {
        UUID id = UUID.randomUUID();
        UUID baseVersion = UUID.randomUUID();
        String clazz = MyAccount.class.getSimpleName();
        common = new MessageDataHeaderDto(id, baseVersion, null, clazz);
        left = new MessageDataHeaderDto(id, UUID.randomUUID(), baseVersion, clazz);
        right = new MessageDataHeaderDto(id, UUID.randomUUID(), baseVersion, clazz);
    }

    @Test
    public void testEmpty3way() {
        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 0);
    }

    @Test
    public void testEmpty2way() {
        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 0);
    }

    @Test
    public void testAddLeft3way() {
        left.getAllowRead().add(leftUUID);

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 1);
        Assert.assertTrue(result.getAllowRead().contains(leftUUID));
    }

    @Test
    public void testAddLeft2way() {
        left.getAllowRead().add(leftUUID);

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 1);
        Assert.assertTrue(result.getAllowRead().contains(leftUUID));
    }

    @Test
    public void testRight3way() {
        right.getAllowRead().add(rightUUID);

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 1);
        Assert.assertTrue(result.getAllowRead().contains(rightUUID));
    }

    @Test
    public void testRight2way() {
        right.getAllowRead().add(rightUUID);

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 1);
        Assert.assertTrue(result.getAllowRead().contains(rightUUID));
    }
    
    @Test
    public void testAddLeftAndRight3way() {
        left.getAllowRead().add(leftUUID);
        right.getAllowRead().add(rightUUID);

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 2);
        Assert.assertTrue(result.getAllowRead().contains(leftUUID));
        Assert.assertTrue(result.getAllowRead().contains(rightUUID));
    }

    @Test
    public void testAddLeftAndRight2way() {
        left.getAllowRead().add(leftUUID);
        right.getAllowRead().add(rightUUID);

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 2);
        Assert.assertTrue(result.getAllowRead().contains(leftUUID));
        Assert.assertTrue(result.getAllowRead().contains(rightUUID));
    }

    @Test
    public void testSubtractRight3way() {
        common.getAllowRead().add(leftUUID);
        common.getAllowRead().add(rightUUID);

        left.getAllowRead().add(leftUUID);
        left.getAllowRead().add(rightUUID);

        right.getAllowRead().add(leftUUID);

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 1);
        Assert.assertTrue(result.getAllowRead().contains(leftUUID));
    }

    @Test
    public void testSubtractRight2way() {
        common.getAllowRead().add(leftUUID);
        common.getAllowRead().add(rightUUID);

        left.getAllowRead().add(leftUUID);
        left.getAllowRead().add(rightUUID);

        right.getAllowRead().add(leftUUID);

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 1);
        Assert.assertTrue(result.getAllowRead().contains(leftUUID));
    }

    @Test
    public void testSubtractLeft3way() {
        common.getAllowRead().add(leftUUID);
        common.getAllowRead().add(rightUUID);

        left.getAllowRead().add(rightUUID);

        right.getAllowRead().add(leftUUID);
        right.getAllowRead().add(rightUUID);

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 1);
        Assert.assertTrue(result.getAllowRead().contains(rightUUID));
    }

    @Test
    public void testSubtractLeft2way() {
        common.getAllowRead().add(leftUUID);
        common.getAllowRead().add(rightUUID);

        left.getAllowRead().add(rightUUID);

        right.getAllowRead().add(leftUUID);
        right.getAllowRead().add(rightUUID);

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 1);
        Assert.assertTrue(result.getAllowRead().contains(rightUUID));
    }

    @Test
    public void testSubtractLeftAndRight3way() {
        common.getAllowRead().add(leftUUID);
        common.getAllowRead().add(rightUUID);

        left.getAllowRead().add(rightUUID);

        right.getAllowRead().add(leftUUID);

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 0);
    }

    @Test
    public void testSubtractLeftAndRight2way() {
        common.getAllowRead().add(leftUUID);
        common.getAllowRead().add(rightUUID);

        left.getAllowRead().add(rightUUID);

        right.getAllowRead().add(leftUUID);

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 0);
    }

    @Test
    public void testAdd10000_3way() {
        for (int n = 0; n < 10000; n++) {
            common.getAllowRead().add(commonUUID);
        }

        for (int n = 0; n < 10000; n++) {
            left.getAllowRead().add(leftUUID);
        }

        for (int n = 0; n < 10000; n++) {
            right.getAllowRead().add(rightUUID);
        }

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 2);
    }

    @Test
    public void testAdd10000_2way() {
        for (int n = 0; n < 10000; n++) {
            common.getAllowRead().add(commonUUID);
        }

        for (int n = 0; n < 10000; n++) {
            left.getAllowRead().add(leftUUID);
        }

        for (int n = 0; n < 10000; n++) {
            right.getAllowRead().add(rightUUID);
        }

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 2);
    }

    @Test
    public void testEmptyFromNull3way() {
        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeThreeWay(null, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 0);
    }

    @Test
    public void testEmptyFromNull2way() {
        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeApply(null, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 0);
    }

    @Test
    public void testAddLeftFromNull3wayA() {
        left.getAllowRead().add(leftUUID);

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeThreeWay(null, left, null);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 1);
        Assert.assertTrue(result.getAllowRead().contains(leftUUID));
    }

    @Test
    public void testAddLeftFromNull2wayA() {
        left.getAllowRead().add(leftUUID);

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeApply(null, left, null);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 1);
        Assert.assertTrue(result.getAllowRead().contains(leftUUID));
    }

    @Test
    public void testAddLeftFromNull3wayB() {
        Set<String> left = new HashSet<>();
        left.add(leftUUID);

        Set<String> result = (Set<String>)merger.mergeThreeWay(null, left, null);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.size() == 1);
        Assert.assertTrue(result.contains(leftUUID));
    }

    @Test
    public void testAddLeftFromNull2wayB() {
        Set<String> left = new HashSet<>();
        left.add(leftUUID);

        Set<String> result = (Set<String>)merger.mergeApply(null, left, null);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.size() == 1);
        Assert.assertTrue(result.contains(leftUUID));
    }

    @Test
    public void testRightFromNull3way() {
        right.getAllowRead().add(rightUUID);

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeThreeWay(null, null, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 1);
        Assert.assertTrue(result.getAllowRead().contains(rightUUID));
    }

    @Test
    public void testRightFromNull2way() {
        right.getAllowRead().add(rightUUID);

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeApply(null, null, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);
        Assert.assertTrue(result.getAllowRead().size() == 1);
        Assert.assertTrue(result.getAllowRead().contains(rightUUID));
    }

    @Test
    public void testAddLeftAndRightFromNull3way() {
        left.getAllowRead().add(leftUUID);
        right.getAllowRead().add(rightUUID);

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeThreeWay(null, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        Assert.assertTrue(result.getAllowRead().size() == 2);
        Assert.assertTrue(result.getAllowRead().contains(leftUUID));
        Assert.assertTrue(result.getAllowRead().contains(rightUUID));
    }

    @Test
    public void testAddLeftAndRightFromNull2way() {
        left.getAllowRead().add(leftUUID);
        right.getAllowRead().add(rightUUID);

        MessageDataHeaderDto result = (MessageDataHeaderDto)merger.mergeApply(null, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assert.assertNotNull(result);

        Assert.assertTrue(result.getAllowRead().size() == 2);
        Assert.assertTrue(result.getAllowRead().contains(leftUUID));
        Assert.assertTrue(result.getAllowRead().contains(rightUUID));
    }
}
