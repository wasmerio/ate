package com.tokera.ate.configuration;

/**
 * Common constants used throughout the ATE data store systems.
 * These are values that are unlikely to change.
 */
public class AteConstants {

    public static final Long GIGABYTE = 1073741824L;
    public static final Long MEGABYTE = 1048576L;

    public static final int TIME_DAY_IN_SECONDS = 60 * 60 * 24;
    public static final int TIME_WEEK_IN_SECONDS = TIME_DAY_IN_SECONDS * 7;
    public static final int TIME_YEAR_IN_SECONDS = TIME_DAY_IN_SECONDS * 365;

    //public static final int DefaultTokenExpiresForWeb = 12 * 60;
    public static final int DefaultTokenExpiresForWeb = 0;
}
