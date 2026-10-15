# RaTeX Maven repository (ExamGOAL fork)

Static Maven repository written by `.github/workflows/release-android.yml`.

```kotlin
repositories {
    maven { url = uri("https://raw.githubusercontent.com/examgoal/RaTeX/maven") }
}
dependencies {
    implementation("com.examgoal:ratex-android:0.1.14-eg.3")
}
```
