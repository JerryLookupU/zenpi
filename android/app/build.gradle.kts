plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("io.github.takahirom.roborazzi") version "1.32.2"
}
val verifyRemoteSchema by tasks.registering(Exec::class) {
    workingDir(rootProject.projectDir.parentFile)
    commandLine("python3", "tools/generate_stage2_android_schema.py", "--check")
}
val verifyPairingEntry by tasks.registering(Exec::class) {
    workingDir(rootProject.projectDir.parentFile)
    commandLine("python3", "tools/validate_stage2_android_entry.py")
}
tasks.named("preBuild") { dependsOn(verifyRemoteSchema, verifyPairingEntry) }
// Rust interop tests (tests/remote_android_*interop.rs) launch JUnitCore on
// the freshly compiled JVM unit-test classes; this records that classpath.
val writeInteropClasspath by tasks.registering {
    val unitTests = tasks.named<Test>("testDebugUnitTest")
    val classpath = unitTests.map { it.classpath }
    val output = layout.buildDirectory.file("interop/classpath.txt")
    dependsOn(classpath)
    outputs.file(output)
    doLast {
        output.get().asFile.writeText(classpath.get().files.joinToString(File.pathSeparator))
    }
}
android {
    namespace = "zenpi"
    compileSdk = 35
    defaultConfig {
        applicationId = "app.zenpi.android"
        minSdk = 31
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }
    buildFeatures { compose = true }
    composeOptions { kotlinCompilerExtensionVersion = "1.5.14" }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }
    sourceSets.getByName("test").resources.srcDir("../../fuzz/remote_protocol")
    // Robolectric renders the portrait screenshot tests (zenpi.screenshot) against merged resources.
    testOptions { unitTests.isIncludeAndroidResources = true }
    // Publishing is opt-in: CI/operators provide the keystore outside the
    // repository.  Without all four values Gradle leaves release unsigned so
    // local builds never depend on or embed a credential.
    val operatorKeystore = System.getenv("ZENPI_ANDROID_KEYSTORE")
    val operatorKeyAlias = System.getenv("ZENPI_ANDROID_KEY_ALIAS")
    val operatorStorePassword = System.getenv("ZENPI_ANDROID_STORE_PASSWORD")
    val operatorKeyPassword = System.getenv("ZENPI_ANDROID_KEY_PASSWORD")
    val signingValues = listOf(operatorKeystore, operatorKeyAlias, operatorStorePassword, operatorKeyPassword)
    require(signingValues.all { it.isNullOrBlank() } || signingValues.all { !it.isNullOrBlank() }) {
        "ZENPI_ANDROID_* signing configuration must be supplied all together"
    }
    if (signingValues.all { !it.isNullOrBlank() }) {
        signingConfigs.create("operatorRelease") {
            storeFile = file(requireNotNull(operatorKeystore))
            this.keyAlias = requireNotNull(operatorKeyAlias)
            this.storePassword = requireNotNull(operatorStorePassword)
            this.keyPassword = requireNotNull(operatorKeyPassword)
        }
    }
    buildTypes {
        release {
            signingConfig = signingConfigs.findByName("operatorRelease")
            isMinifyEnabled = false
            isDebuggable = false
        }
    }
    packaging { resources.excludes += "/META-INF/{AL2.0,LGPL2.1}" }
}
// Portrait goldens: `recordRoborazziDebug` rewrites them, `verifyRoborazziDebug` (G-ANDROID) fails on drift.
roborazzi { outputDir.set(file("src/test/resources/golden")) }
dependencies {
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.4")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.4")
    implementation("androidx.work:work-runtime-ktx:2.9.1")
    implementation(platform("androidx.compose:compose-bom:2024.09.03"))
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("androidx.compose.ui:ui")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.1")
    implementation("com.journeyapps:zxing-android-embedded:4.3.0")
    implementation("com.google.code.gson:gson:2.11.0")
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.robolectric:robolectric:4.14.1")
    testImplementation("io.github.takahirom.roborazzi:roborazzi:1.32.2")
    testImplementation("io.github.takahirom.roborazzi:roborazzi-compose:1.32.2")
    testImplementation(platform("androidx.compose:compose-bom:2024.09.03"))
    testImplementation("androidx.compose.ui:ui-test-junit4")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
    androidTestImplementation(platform("androidx.compose:compose-bom:2024.09.03"))
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}
