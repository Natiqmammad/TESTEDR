package com.apexforge;

import org.apexforge.AfmlExport;

public class Hello {
    @AfmlExport(signature = "fn greet_java() -> str")
    public static String greet_java() {
        return "hello from java";
    }

    @AfmlExport(signature = "fn sum(i32, i32) -> i32")
    public static int sum(int a, int b) {
        return a + b;
    }
}
