package org.apexforge;

import java.lang.annotation.*;

@Retention(RetentionPolicy.SOURCE)
@Target(ElementType.METHOD)
public @interface AfmlExport {
    String signature();
}
