// Kubuno Photos — the native client for the photos module. A consumer app like
// mail and maps: it reuses :core-account (shared accounts, the com.kubuno
// authenticator) and :core-ui (design tokens), and never holds a refresh token.
plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.ksp)
    alias(libs.plugins.kotlin.serialization)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.hilt)
}

android {
    namespace = "com.kubuno.photos"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.kubuno.photos.android"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
    }

    // Release signing is opt-in: pass -PkubunoKeystore=… (and the passwords) to
    // sign, otherwise the release APK is unsigned (what the CI publishes). EVERY
    // Kubuno app MUST be signed with the SAME certificate — the shared-account
    // model grants access by matching signature, so a differently signed build
    // is refused a borrowed token and every screen ends up at 401.
    val keystorePath = (findProperty("kubunoKeystore") as String?)?.takeIf { it.isNotBlank() }
    val keystoreFile = keystorePath?.let { rootProject.file(it) }
    signingConfigs {
        if (keystoreFile != null && keystoreFile.exists()) {
            create("release") {
                storeFile = keystoreFile
                storePassword = findProperty("kubunoKeystorePassword") as String?
                keyAlias = findProperty("kubunoKeyAlias") as String?
                keyPassword = findProperty("kubunoKeyPassword") as String?
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            signingConfig = signingConfigs.findByName("release")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
}

dependencies {
    implementation(libs.kubuno.core.api)
    implementation(libs.kubuno.core.account)
    implementation(libs.kubuno.core.ui)
    implementation(libs.kubuno.core.viewer)

    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.appcompat)

    val composeBom = platform(libs.compose.bom)
    implementation(composeBom)
    implementation(libs.compose.ui)
    implementation(libs.compose.ui.tooling.preview)
    implementation(libs.compose.material3)
    implementation(libs.compose.material.icons)
    debugImplementation(libs.compose.ui.tooling)

    implementation(libs.activity.compose)
    implementation(libs.lifecycle.runtime.compose)
    implementation(libs.lifecycle.viewmodel.compose)

    implementation(libs.hilt.android)
    ksp(libs.hilt.compiler)
    implementation(libs.hilt.navigation.compose)

    implementation(libs.coroutines.android)
    implementation(libs.kotlinx.serialization.json)
    implementation(libs.retrofit.kotlinx.serialization)

    implementation(libs.coil.compose)
    implementation(libs.coil.network.okhttp)

    testImplementation(libs.junit)
    testImplementation(libs.coroutines.test)
}
