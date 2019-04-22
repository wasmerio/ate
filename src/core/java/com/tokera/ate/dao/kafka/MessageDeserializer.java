/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dao.kafka;

import java.nio.ByteBuffer;
import java.util.Map;

import com.tokera.ate.dao.msg.MessageBase;
import org.apache.kafka.common.serialization.Deserializer;

/**
 * Kafka deserializer used to handle the main message flatbuffers
 */
public class MessageDeserializer implements Deserializer<MessageBase> {
    
    @Override
    public void configure(Map<String, ?> map, boolean bln) {
    }

    @Override
    public MessageBase deserialize(String string, byte[] bytes) {
        ByteBuffer buf = ByteBuffer.wrap(bytes);
        return MessageBase.getRootAsMessageBase(buf);
    }

    @Override
    public void close() {
    }
}