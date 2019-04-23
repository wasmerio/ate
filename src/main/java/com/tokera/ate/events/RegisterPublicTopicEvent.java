package com.tokera.ate.events;

public class RegisterPublicTopicEvent {
    private String name;

    public RegisterPublicTopicEvent(String name) {
        this.name = name;
    }

    public String getName() {
        return name;
    }

    public void setName(String name) {
        this.name = name;
    }
}
