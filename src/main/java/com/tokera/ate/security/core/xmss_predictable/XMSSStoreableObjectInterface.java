package com.tokera.ate.security.core.xmss_predictable;

/**
 * Interface for XMSS objects that need to be storeable as a byte array.
 */
public interface XMSSStoreableObjectInterface
{

    /**
     * Create byte representation of object.
     *
     * @return Byte representation of object.
     */
    byte[] toByteArray();
}