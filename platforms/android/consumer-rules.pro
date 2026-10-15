# JNI: libratex_ffi binds io.ratex.RaTeXEngine's native methods by name and calls
# RaTeXTextMeasurer.measure(String, int, float[]) on host measurers by name.
-keep class io.ratex.RaTeXEngine { *; }
-keep interface io.ratex.RaTeXTextMeasurer { *; }
-keep class * implements io.ratex.RaTeXTextMeasurer {
    void measure(java.lang.String, int, float[]);
}
# kotlinx.serialization models of the display-list JSON
-keep class io.ratex.DisplayList { *; }
-keep class io.ratex.DisplayItem { *; }
-keep class io.ratex.DisplayItem$* { *; }
-keep class io.ratex.PathCommand { *; }
-keep class io.ratex.PathCommand$* { *; }
-keep class io.ratex.RaTeXColor { *; }
