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

import com.tokera.ate.dao.CountLong;
import com.tokera.ate.io.merge.DataMerger;
import com.tokera.ate.test.dao.MyAccount;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;

/**
 *
 * @author John
 */
@TestInstance(TestInstance.Lifecycle.PER_METHOD)
public class MergeCounters {

    private DataMerger merger = new DataMerger();

    MyAccount common = new MyAccount();
    MyAccount left = new MyAccount();
    MyAccount right = new MyAccount();

    @Test
    public void testLeftIncrement() {
        common.counter = new CountLong(0L);
        left.counter = new CountLong(1L);
        right.counter = new CountLong(0L);

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(new CountLong(1L), result.counter);
    }

    @Test
    public void testRightIncrement() {
        common.counter = new CountLong(0L);
        left.counter = new CountLong(0L);
        right.counter = new CountLong(1L);

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(new CountLong(1L), result.counter);
    }


    @Test
    public void testDoubleIncrement() {
        common.counter = new CountLong(0L);
        left.counter = new CountLong(1L);
        right.counter = new CountLong(1L);

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(new CountLong(2L), result.counter);
    }

    @Test
    public void testLeftDecrement() {
        common.counter = new CountLong(0L);
        left.counter = new CountLong(-1L);
        right.counter = new CountLong(0L);

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(new CountLong(-1L), result.counter);
    }

    @Test
    public void testRightDecrement() {
        common.counter = new CountLong(0L);
        left.counter = new CountLong(0L);
        right.counter = new CountLong(-1L);

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(new CountLong(-1L), result.counter);
    }

    @Test
    public void testDoubleDecrement() {
        common.counter = new CountLong(0L);
        left.counter = new CountLong(-1L);
        right.counter = new CountLong(-1L);

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(new CountLong(-2L), result.counter);
    }

    @Test
    public void testCancelOut() {
        common.counter = new CountLong(0L);
        left.counter = new CountLong(-1L);
        right.counter = new CountLong(1L);

        MyAccount result = merger.mergeThreeWay(common, left, right);
        assert result != null : "@AssumeAssertion(nullness): Must not be null";
        Assertions.assertEquals(new CountLong(0L), result.counter);
    }
}
