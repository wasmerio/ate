package com.tokera.ate.providers;

import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.SerializationFeature;
import com.fasterxml.jackson.module.scala.DefaultScalaModule;
import org.jboss.resteasy.plugins.providers.jackson.ResteasyJackson2Provider;

import javax.ws.rs.Consumes;
import javax.ws.rs.Produces;
import javax.ws.rs.ext.Provider;

@Provider
@Consumes({"application/json", "application/*+json", "text/json"})
@Produces({"application/json", "application/*+json", "text/json"})
public class TokeraResteasyJackson2Provider extends ResteasyJackson2Provider {

    public TokeraResteasyJackson2Provider() {
        ObjectMapper mapper = new ObjectMapper();
        TokeraResteasyJackson2Provider.configureMapper(mapper);
        super.setMapper(mapper);
    }

    public static void configureMapper(ObjectMapper mapper) {
        mapper.registerModule(new DefaultScalaModule());
        mapper.setSerializationInclusion(JsonInclude.Include.NON_NULL);
        mapper.setSerializationInclusion(JsonInclude.Include.NON_DEFAULT);
        mapper.configure(SerializationFeature.WRITE_DATES_AS_TIMESTAMPS, false);
        mapper.configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);
    }
}
