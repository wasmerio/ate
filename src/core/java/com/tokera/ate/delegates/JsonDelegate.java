package com.tokera.ate.delegates;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;

import javax.enterprise.context.ApplicationScoped;

@ApplicationScoped
public class JsonDelegate {

    public Gson gson() {
        return JsonDelegate.createGson();
    }

    public static Gson createGson() {
        Gson gson = new GsonBuilder()
                .excludeFieldsWithoutExposeAnnotation()
                .setDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSSZ")
                .setPrettyPrinting()
                .create();
        return gson;
    }
}
