package com.tokera.ate.configuration;

/**
 * Common constants used throughout the ATE data store systems.
 * These are values that are unlikely to change.
 */
public class AteConstants {

    public static final Long GIGABYTE = 1073741824L;
    public static final Long MEGABYTE = 1048576L;
    
    public static final String PROPERTY_ARGS_IP = "args.ip";
    public static final String PROPERTY_ARGS_PORT = "args.port";

    public static final String PROPERTY_LOG4J_SYSTEM = "log4j.configuration";
    public static final String PROPERTY_KAFKA_SYSTEM = "kafka.configuration";
    public static final String PROPERTY_ZOOKEEPER_SYSTEM = "zookeeper.configuration";
    public static final String PROPERTY_CONSUMER_SYSTEM = "consumer.configuration";
    public static final String PROPERTY_PRODUCER_SYSTEM = "producer.configuration";
    public static final String PROPERTY_TOPIC_DAO_SYSTEM = "topic.dao.configuration";
    public static final String PROPERTY_TOPIC_IO_SYSTEM = "topic.io.configuration";
    public static final String PROPERTY_TOPIC_PUBLISH_SYSTEM = "topic.publish.configuration";

    public static final String PROPERTIES_FILE_LOG4J = "log4j.properties";
    public static final String PROPERTIES_FILE_KAFKA = "kafka.properties";
    public static final String PROPERTIES_FILE_ZOOKEEPER = "zookeeper.properties";    
    public static final String PROPERTIES_FILE_CONSUMER = "consumer.properties";
    public static final String PROPERTIES_FILE_PRODUCER = "producer.properties";
    public static final String PROPERTIES_FILE_TOPIC_DAO = "topic.dao.properties";
    public static final String PROPERTIES_FILE_TOPIC_IO = "topic.io.properties";
    public static final String PROPERTIES_FILE_TOPIC_PUBLISH = "topic.publish.properties";

    public static final String RUNTIME_CONTEXT_PROPERTY = "runtime.requestContext";
    public static final String RUNTIME_CONTEXT_PRODUCTION = "production";
    public static final String RUNTIME_CONTEXT_DEVELOPMENT = "development";
    public static final String RUNTIME_CONTEXT_COMMON = "common";

    public static final boolean ENABLE_REGISTER_VERIFY = false;

    public static final int TIME_DAY_IN_SECONDS = 60 * 60 * 24;
    public static final int TIME_WEEK_IN_SECONDS = TIME_DAY_IN_SECONDS * 7;
    public static final int TIME_YEAR_IN_SECONDS = TIME_DAY_IN_SECONDS * 365;

    //public static final int DefaultTokenExpiresForWeb = 12 * 60;
    public static final int DefaultTokenExpiresForWeb = 0;
}
