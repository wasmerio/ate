package com.tokera.ate.common;

import com.tokera.ate.configuration.AteConstants;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import javax.ws.rs.WebApplicationException;
import java.io.InputStream;

/**
 * This class works out what requestContext (production, development) you are in and
 * automatically returns the resource in the right requestContext.
 *
 * Production resource directory: src/main/resources/production
 * Development resource directory: src/main/resources/development
 *
 * @author jonhanlee
 */
public class ResourceUtils {
	private static final Logger LOG = LoggerFactory.getLogger(ResourceUtils.class);

	private static String runtimeContext = AteConstants.RUNTIME_CONTEXT_PRODUCTION;

	static {
		String runtimeContextValue = System.getProperty(AteConstants.RUNTIME_CONTEXT_PROPERTY);
		if (runtimeContextValue != null) {
			runtimeContext = runtimeContextValue;
		}
		LOG.info("Runtime Context:" + runtimeContext);
	}

	public static String getRuntimeContext() {
		return runtimeContext;
	}

	/**
	 * Returns the runtime requestContext adjusted resource	 *
	 * @param url the unadjusted URL to the intended resources
	 * @return the input stream of the resource
	 */
	public static InputStream getRuntimeContextAdjustedResourceAsStream(String url) {
		InputStream ret = ResourceUtils.class.getResourceAsStream("/" + ResourceUtils.getRuntimeContext() + "/" + url);
		if (ret == null) throw new WebApplicationException("InputStream could not be created for this resource [" + url + "].");
		return ret;
	}
}
