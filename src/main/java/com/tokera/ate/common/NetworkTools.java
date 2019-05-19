package com.tokera.ate.common;

import org.checkerframework.checker.nullness.qual.Nullable;

import java.net.InetAddress;
import java.net.NetworkInterface;
import java.net.SocketException;
import java.net.UnknownHostException;
import java.util.Collections;
import java.util.HashSet;
import java.util.Set;

public class NetworkTools {

    public static Set<String> getMyNetworkAddresses() {
        HashSet<String> myAddresses = new HashSet<>();
        try {
            for (NetworkInterface net : Collections.list(NetworkInterface.getNetworkInterfaces())) {
                for (InetAddress addr : Collections.list(net.getInetAddresses())) {
                    myAddresses.add(addr.toString());
                }
            }
        } catch (SocketException e) {
            try {
                myAddresses.add(InetAddress.getLocalHost().toString());
            } catch (UnknownHostException e1) {
                throw new RuntimeException("Failed to determine the IP address of this machine.", e1);
            }
        }
        return myAddresses;
    }

    public static @Nullable Integer extractPortFromBootstrap(@Nullable String bootstrap) {
        if (bootstrap == null) return null;
        String[] comps = bootstrap.split(":");
        if (comps.length < 2) return null;
        return Integer.parseInt(comps[1]);
    }

    public static Integer extractPortFromBootstrapOrThrow(String bootstrap) {
        Integer ret = extractPortFromBootstrap(bootstrap);
        if (ret == null) {
            throw new RuntimeException("Failed to determine the port from the address [" + bootstrap + "] - ensure a port number of post-fixed on the end separated with a colon.");
        }
        return ret;
    }
}
