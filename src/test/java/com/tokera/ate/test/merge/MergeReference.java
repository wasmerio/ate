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

/**
 *
 * @author John
 */
@TestInstance(TestInstance.Lifecycle.PER_METHOD)
public class MergeReference {

    private DataMerger merger = new DataMerger();

    MyAccount common = new MyAccount();
    MyAccount left = new MyAccount();
    MyAccount right = new MyAccount();

    @Test
    public void testRightNull3way() {
        Object result = merger.mergeThreeWay(common, left, null);
        Assert.assertTrue(result == null);
    }

    @Test
    public void testRightNull2way() {
        Object result = merger.mergeApply(common, left, null);
        Assert.assertTrue(result != null);
    }

    @Test
    public void testLeftNull3way() {
        Object result = merger.mergeThreeWay(common, null, right);
        Assert.assertTrue(result == null);
    }

    @Test
    public void testLeftNull2way() {
        Object result = merger.mergeApply(common, null, right);
        Assert.assertTrue(result == null);
    }

    @Test
    public void testLeftAndRightNull3way() {
        Object result = merger.mergeThreeWay(common, null, null);
        Assert.assertTrue(result == null);
    }

    @Test
    public void testLeftAndRightNull2way() {
        Object result = merger.mergeApply(common, null, null);
        Assert.assertTrue(result == null);
    }

    @Test
    public void testLeftConstruct3way() {
        Object result = merger.mergeThreeWay(null, left, null);
        Assert.assertTrue(result != null);
    }

    @Test
    public void testLeftConstruct2way() {
        Object result = merger.mergeApply(null, left, null);
        Assert.assertTrue(result != null);
    }

    @Test
    public void testRightConstruct3way() {
        Object result = merger.mergeThreeWay(null, null, right);
        Assert.assertTrue(result != null);
    }

    @Test
    public void testRightConstruct2way() {
        Object result = merger.mergeApply(null, null, right);
        Assert.assertTrue(result != null);
    }

    @Test
    public void testLeftAndRightConstruct3way() {
        Object result = merger.mergeThreeWay(null, left, right);
        Assert.assertTrue(result != null);
    }

    @Test
    public void testLeftAndRightConstruct2way() {
        Object result = merger.mergeApply(null, left, right);
        Assert.assertTrue(result != null);
    }
}
