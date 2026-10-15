import java.io.File

plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android") version "1.9.24"
    id("org.jetbrains.kotlin.plugin.serialization") version "1.9.24"
    id("com.vanniktech.maven.publish") version "0.30.0"
}

android {
    namespace = "io.ratex"
    compileSdk = 34

    defaultConfig {
        minSdk = 21   // Android 5.0+, broad device support
        targetSdk = 34

        // Package the native .so files built by build-android.sh
        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64", "x86")
        }
        // native code binds io.ratex.RaTeXEngine and calls RaTeXTextMeasurer.measure by name
        consumerProguardFiles("consumer-rules.pro")
    }

    sourceSets["main"].jniLibs.srcDirs("src/main/jniLibs")
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
}

dependencies {
    implementation("androidx.annotation:annotation:1.7.1")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.3")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.3")
}

// CI（release-android.yml）传入 -PlibraryVersion；本地默认与根目录 VERSION 一致
val libraryVersion = project.findProperty("libraryVersion")?.toString()?.takeIf { it.isNotBlank() }
    ?: File(rootProject.rootDir, "../../VERSION").normalize().readText().trim()

// ExamGOAL fork: published to the GitHub-hosted Maven repository on the fork's
// `maven` branch (https://raw.githubusercontent.com/examgoal/RaTeX/maven) by
// release-android.yml; `-PmavenRepoDir=<dir>` picks where the repo is written.
// Maven Central publishing still works when Sonatype credentials and a signing
// key are configured.
val mavenRepoDir = project.findProperty("mavenRepoDir")?.toString()
    ?: layout.buildDirectory.dir("maven-repo").get().asFile.path

publishing {
    repositories {
        maven {
            name = "githubMaven"
            url = uri(mavenRepoDir)
        }
    }
}

mavenPublishing {
    publishToMavenCentral(com.vanniktech.maven.publish.SonatypeHost.CENTRAL_PORTAL, automaticRelease = true)
    if (project.hasProperty("signingInMemoryKey")) signAllPublications()
    coordinates("com.examgoal", "ratex-android", libraryVersion)

    pom {
        name.set("RaTeX Android (ExamGOAL fork)")
        description.set("Native Android rendering of LaTeX math (Canvas + KaTeX fonts) with a MathJax 3 compatibility mode")
        url.set("https://github.com/examgoal/RaTeX")
        licenses {
            license {
                name.set("MIT")
                url.set("https://opensource.org/licenses/MIT")
            }
        }
        scm {
            url.set("https://github.com/examgoal/RaTeX")
            connection.set("scm:git:git://github.com/examgoal/RaTeX.git")
            developerConnection.set("scm:git:ssh://git@github.com/examgoal/RaTeX.git")
        }
        developers {
            developer {
                name.set("RaTeX Contributors")
                url.set("https://github.com/erweixin/RaTeX")
            }
            developer {
                name.set("ExamGOAL")
                url.set("https://github.com/examgoal/RaTeX")
            }
        }
    }
}
