plugins { id("com.android.application"); id("org.jetbrains.kotlin.android") }
android {
    namespace = "org.linmic.app"
    compileSdk = 36
    ndkVersion = "28.2.13676358"
    defaultConfig {
        applicationId = "org.linmic.app"
        minSdk = 26
        targetSdk = 36
        versionCode = 3
        versionName = "0.2.1"
        ndk { abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64") }
        externalNativeBuild { cmake { arguments += "-DANDROID_STL=c++_shared"; cppFlags += listOf("-std=c++17", "-Wall", "-Wextra") } }
    }
    buildFeatures { prefab = true }
    externalNativeBuild { cmake { path = file("../native/CMakeLists.txt"); version = "3.22.1" } }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    kotlinOptions { jvmTarget = "17" }
    signingConfigs {
        create("release") {
            val keyPath = System.getenv("LINMIC_KEYSTORE")
            if (keyPath != null) {
                storeFile = file(keyPath)
                storePassword = System.getenv("LINMIC_STORE_PASSWORD")
                keyAlias = System.getenv("LINMIC_KEY_ALIAS")
                keyPassword = System.getenv("LINMIC_KEY_PASSWORD")
            }
        }
    }
    buildTypes { release { isMinifyEnabled = false; if (System.getenv("LINMIC_KEYSTORE") != null) signingConfig = signingConfigs.getByName("release") } }
    lint { abortOnError = true }
}
dependencies {
    implementation("com.google.oboe:oboe:1.10.0")
    testImplementation("junit:junit:4.13.2")
}
val buildPairing by tasks.registering(Exec::class) {
    workingDir(rootProject.projectDir.parentFile)
    commandLine("python3", "scripts/build-android-pairing.py")
    inputs.dir(rootProject.file("../crates/linmic-pairing"))
    inputs.file(rootProject.file("../Cargo.lock"))
    outputs.dir(rootProject.file("native/rust"))
}
tasks.configureEach {
    if (name == "preBuild" || name.startsWith("configureCMake")) dependsOn(buildPairing)
}
