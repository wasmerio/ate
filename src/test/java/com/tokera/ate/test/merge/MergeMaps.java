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

import com.tokera.ate.common.MapTools;
import com.tokera.ate.test.dao.MyAccount;
import com.tokera.ate.io.merge.DataMerger;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;

import java.util.HashMap;
import java.util.Map;
import java.util.UUID;

/**
 *
 * @author John
 */
@TestInstance(TestInstance.Lifecycle.PER_METHOD)
public class MergeMaps {

    private DataMerger merger = new DataMerger();

    UUID commonVal = UUID.randomUUID();
    UUID leftVal = UUID.randomUUID();
    UUID rightVal = UUID.randomUUID();

    MyAccount common = new MyAccount();
    MyAccount left = new MyAccount();
    MyAccount right = new MyAccount();

    public MergeMaps() {
    }

    @Test
    public void testEmpty3way() {
        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 0);

        result = merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 0);
    }

    @Test
    public void testEmpty2way() {
        MyAccount result = merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 0);
    }

    @Test
    public void testAddLeft3way() {
        left.textFiles.put("1", leftVal);

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 1);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(leftVal, id);
    }

    @Test
    public void testAddLeft2way() {
        left.textFiles.put("1", leftVal);

        MyAccount result = merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 1);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(leftVal, id);
    }

    @Test
    public void testAddRight3way() {
        right.textFiles.put("2", rightVal);

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 1);

        UUID id = MapTools.getOrNull(result.textFiles, "2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testAddRight2way() {
        right.textFiles.put("2", rightVal);

        MyAccount result = merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 1);

        UUID id = MapTools.getOrNull(result.textFiles, "2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }
    
    @Test
    public void testAddLeftAndRight3way() {
        left.textFiles.put("1", leftVal);
        right.textFiles.put("2", rightVal);

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 2);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(leftVal, id);

        id = MapTools.getOrNull(result.textFiles, "2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testAddLeftAndRight2way() {
        left.textFiles.put("1", leftVal);
        right.textFiles.put("2", rightVal);

        MyAccount result = merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 2);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(leftVal, id);

        id = MapTools.getOrNull(result.textFiles, "2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testAddLeftAndRightBias3way() {
        left.textFiles.put("1", leftVal);
        right.textFiles.put("1", rightVal);

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 1);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testAddLeftAndRightBias2way() {
        left.textFiles.put("1", leftVal);
        right.textFiles.put("1", rightVal);

        MyAccount result = merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 1);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(leftVal, id);
    }

    @Test
    public void testReplaceLeft3way() {
        common.textFiles.put("1", commonVal);
        common.textFiles.put("2", commonVal);
        left.textFiles.put("1", leftVal);
        left.textFiles.put("2", commonVal);
        right.textFiles.put("1", commonVal);
        right.textFiles.put("2", commonVal);

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 2);

        UUID id = MapTools.getOrNull(result.textFiles, "2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(commonVal, id);

        id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(leftVal, id);
    }

    @Test
    public void testReplaceLeft2way() {
        common.textFiles.put("1", commonVal);
        common.textFiles.put("2", commonVal);
        left.textFiles.put("1", leftVal);
        left.textFiles.put("2", commonVal);
        right.textFiles.put("1", commonVal);
        right.textFiles.put("2", commonVal);

        MyAccount result = merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 2);

        UUID id = MapTools.getOrNull(result.textFiles, "2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(commonVal, id);

        id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(leftVal, id);
    }

    @Test
    public void testReplaceRight3way() {
        common.textFiles.put("1", commonVal);
        common.textFiles.put("2", commonVal);
        left.textFiles.put("1", commonVal);
        left.textFiles.put("2", commonVal);
        right.textFiles.put("1", commonVal);
        right.textFiles.put("2", rightVal);

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 2);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(commonVal, id);

        id = MapTools.getOrNull(result.textFiles, "2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testReplaceRight2way() {
        common.textFiles.put("1", commonVal);
        common.textFiles.put("2", commonVal);
        left.textFiles.put("1", commonVal);
        left.textFiles.put("2", commonVal);
        right.textFiles.put("1", commonVal);
        right.textFiles.put("2", rightVal);

        MyAccount result = merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 2);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(commonVal, id);

        id = MapTools.getOrNull(result.textFiles, "2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testReplaceLeftAndRight3way() {
        common.textFiles.put("1", commonVal);
        common.textFiles.put("2", commonVal);
        left.textFiles.put("1", leftVal);
        left.textFiles.put("2", commonVal);
        right.textFiles.put("1", commonVal);
        right.textFiles.put("2", rightVal);

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertTrue(result.textFiles.size() == 2);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(leftVal, id);

        id = MapTools.getOrNull(result.textFiles, "2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testReplaceLeftAndRight2way() {
        common.textFiles.put("1", commonVal);
        common.textFiles.put("2", commonVal);
        left.textFiles.put("1", leftVal);
        left.textFiles.put("2", commonVal);
        right.textFiles.put("1", commonVal);
        right.textFiles.put("2", rightVal);

        MyAccount result = merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertTrue(result.textFiles.size() == 2);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(leftVal, id);

        id = MapTools.getOrNull(result.textFiles, "2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testReplaceLeftAndRightBias3way() {
        common.textFiles.put("1", commonVal);
        left.textFiles.put("1", leftVal);
        right.textFiles.put("1", rightVal);

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 1);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testReplaceLeftAndRightBias2way() {
        common.textFiles.put("1", commonVal);
        left.textFiles.put("1", leftVal);
        right.textFiles.put("1", rightVal);

        MyAccount result = merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 1);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(leftVal, id);
    }

    @Test
    public void testSubtractRight3way() {
        common.textFiles.put("1", leftVal);
        common.textFiles.put("2", rightVal);

        left.textFiles.put("1", leftVal);
        left.textFiles.put("2", rightVal);

        right.textFiles.put("1", leftVal);

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 1);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(leftVal, id);
    }

    @Test
    public void testSubtractRight2way() {
        common.textFiles.put("1", leftVal);
        common.textFiles.put("2", rightVal);

        left.textFiles.put("1", leftVal);
        left.textFiles.put("2", rightVal);

        right.textFiles.put("1", leftVal);

        MyAccount result = merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 2);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(leftVal, id);

        id = MapTools.getOrNull(result.textFiles, "2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testSubtractLeft3way() {
        common.textFiles.put("1", leftVal);
        common.textFiles.put("2", rightVal);

        left.textFiles.put("2", rightVal);

        right.textFiles.put("1", leftVal);
        right.textFiles.put("2", rightVal);

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 1);

        UUID id = MapTools.getOrNull(result.textFiles, "2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testSubtractLeft2way() {
        common.textFiles.put("1", leftVal);
        common.textFiles.put("2", rightVal);

        left.textFiles.put("2", rightVal);

        right.textFiles.put("1", leftVal);
        right.textFiles.put("2", rightVal);

        MyAccount result = merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 1);

        UUID id = MapTools.getOrNull(result.textFiles, "2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testSubtractLeftAndRight3way() {
        common.textFiles.put("1", leftVal);
        common.textFiles.put("2", rightVal);

        left.textFiles.put("2", rightVal);

        right.textFiles.put("1", leftVal);

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 0);
    }

    @Test
    public void testSubtractLeftAndRight2wayA() {
        common.textFiles.put("1", leftVal);
        common.textFiles.put("2", rightVal);

        left.textFiles.put("2", rightVal);

        right.textFiles.put("1", leftVal);

        MyAccount result = merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 1);
        Assertions.assertTrue(result.textFiles.containsKey("2"));
        UUID id = result.textFiles.get("2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testSubtractLeftAndRight2wayB() {
        Map<String, UUID> common = new HashMap<>();
        Map<String, UUID> left = new HashMap<>();
        Map<String, UUID> right = new HashMap<>();

        common.put("1", leftVal);
        common.put("2", rightVal);

        left.put("2", rightVal);

        right.put("1", leftVal);

        Map<String, UUID> result = merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.size() == 1);
        Assertions.assertTrue(result.containsKey("2"));
        UUID id = result.get("2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testAdd100003way() {
        for (int n = 0; n < 10000; n++) {
            common.textFiles.put(Integer.toString(n), commonVal);
            left.textFiles.put(Integer.toString(n), commonVal);
        }

        for (int n = 0; n < 5000; n++) {
            right.textFiles.put(Integer.toString(n), leftVal);
        }

        for (int n = 5000; n < 10000; n++) {
            right.textFiles.put(Integer.toString(n), rightVal);
        }

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 10000);
        for (int n = 0; n < 5000; n++) {
            UUID id = MapTools.getOrNull(result.textFiles, Integer.toString(n));
            assert id != null : "@AssumeAssertion(nullness): Must not be null";
            Assertions.assertEquals(leftVal, id);
        }
        for (int n = 5000; n < 10000; n++) {
            UUID id = MapTools.getOrNull(result.textFiles, Integer.toString(n));
            assert id != null : "@AssumeAssertion(nullness): Must not be null";
            Assertions.assertEquals(rightVal, id);
        }
    }

    @Test
    public void testAdd100002way() {
        for (int n = 0; n < 10000; n++) {
            common.textFiles.put(Integer.toString(n), commonVal);
            left.textFiles.put(Integer.toString(n), commonVal);
        }

        for (int n = 0; n < 5000; n++) {
            right.textFiles.put(Integer.toString(n), leftVal);
        }

        for (int n = 5000; n < 10000; n++) {
            right.textFiles.put(Integer.toString(n), rightVal);
        }

        MyAccount result = merger.mergeApply(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 10000);
        for (int n = 0; n < 5000; n++) {
            UUID id = MapTools.getOrNull(result.textFiles, Integer.toString(n));
            assert id != null : "@AssumeAssertion(nullness): Must not be null";
            Assertions.assertEquals(leftVal, id);
        }
        for (int n = 5000; n < 10000; n++) {
            UUID id = MapTools.getOrNull(result.textFiles, Integer.toString(n));
            assert id != null : "@AssumeAssertion(nullness): Must not be null";
            Assertions.assertEquals(rightVal, id);
        }
    }

    @Test
    public void testAddLeftFromNull3way() {
        left.textFiles.put("1", leftVal);

        MyAccount result = merger.mergeThreeWay(null, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 1);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(leftVal, id);
    }

    @Test
    public void testAddLeftFromNull2way() {
        left.textFiles.put("1", leftVal);

        MyAccount result = merger.mergeApply(null, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 1);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(leftVal, id);
    }

    @Test
    public void testAddRightFromNull3way() {
        right.textFiles.put("2", rightVal);

        MyAccount result = merger.mergeThreeWay(null, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertTrue(result.textFiles.size() == 1);

        UUID id = MapTools.getOrNull(result.textFiles, "2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testAddRightFromNull2way() {
        right.textFiles.put("2", rightVal);

        MyAccount result = merger.mergeApply(null, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertTrue(result.textFiles.size() == 1);

        UUID id = MapTools.getOrNull(result.textFiles, "2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testAddLeftAndRightFromNull3way() {
        left.textFiles.put("1", leftVal);
        right.textFiles.put("2", rightVal);

        MyAccount result = merger.mergeThreeWay(null, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertTrue(result.textFiles.size() == 2);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(leftVal, id);

        id = MapTools.getOrNull(result.textFiles, "2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testAddLeftAndRightFromNull2way() {
        left.textFiles.put("1", leftVal);
        right.textFiles.put("2", rightVal);

        MyAccount result = merger.mergeApply(null, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertTrue(result.textFiles.size() == 2);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(leftVal, id);

        id = MapTools.getOrNull(result.textFiles, "2");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testAddLeftAndRightBiasFromNull3way() {
        left.textFiles.put("1", leftVal);
        right.textFiles.put("1", rightVal);

        MyAccount result = merger.mergeThreeWay(null, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 1);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }

    @Test
    public void testAddLeftAndRightBiasFromNull2way() {
        left.textFiles.put("1", leftVal);
        right.textFiles.put("1", rightVal);

        MyAccount result = merger.mergeThreeWay(null, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertNotNull(result);
        Assertions.assertTrue(result.textFiles.size() == 1);

        UUID id = MapTools.getOrNull(result.textFiles, "1");
        assert id != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(rightVal, id);
    }
}
