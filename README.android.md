# Helper Page to Get Started on Android

Install the Arch packages:
```
pacman -S jdk21-openjdk android-tools
```

Initialized Rust:
```
rustup default stable
```

## Android Studio

```
/opt/android-studio
```

```
/opt/android-studio/bin/studio.sh
```

### Android SDK

In Android Studio we configured the SDK location as:
```
$HOME/Android/Sdk
```

In:
Settings → Languages & Frameworks → Android SDK → SDK Tools

We installed:
- Android SDK Command-line Tools
- Android SDK Platform-Tools
- Android SDK Build-Tools
- NDK (Side by side)
- Android Emulator was also available, although we didn't need it

Initially we were missing the command-line tools and NDK, which caused the first:
```
failed to ensure Android environment
```

After installing them, we had:
```
$HOME/Android/Sdk/cmdline-tools/
```
and:
```
$HOME/Android/Sdk/ndk/30.0.15729638
```

## Configure Android environment variables
```
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_SDK_ROOT="$HOME/Android/Sdk"
```

```
export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/cmdline-tools/bin:$PATH"
export PATH="$ANDROID_HOME/emulator:$PATH"
```

## Android device / ADB

We connected the Android phone via USB with USB debugging enabled.

```
adb kill-server
adb start-server
adb devices
```

```
adb shell getprop ro.product.model
```

```
adb shell pm list packages | grep -i knofoo
```

## Rust Android targets

We installed the Android Rust targets:
```
rustup target add \
  aarch64-linux-android \
  armv7-linux-androideabi \
  i686-linux-android \
  x86_64-linux-android
```

## Tauri

```
export JAVA_HOME=/usr/lib/jvm/java-21-openjdk
npx tauri android dev
```

### Fix the npm / tauri script

Our package.json originally had:

```
"scripts": {
  "dev": "vite",
  "build": "vite build",
  "preview": "vite preview"
}
```
but the Tauri Android build needed the tauri npm script.

We added:

```
"scripts": {
  "dev": "vite",
  "build": "vite build",
  "preview": "vite preview",
  "tauri": "tauri"
}
```

## Initialize Tauri Android

```
npx tauri android init
```

## Run the app

```
npx tauri android dev
```

## Debugging if there are errors

```
npx tauri android dev --verbose 2>&1 | tee /tmp/tauri-android.log

grep -n -E 'npm ERR|npm error|error:|ERROR|FAILURE|Caused by:|Could not|failed' /tmp/tauri-android.log | tail -80
```

JDK 21 is well-supported by Gradle 8.14 (class file version 65). It's high enough to avoid the "too old" issues but low enough to avoid the "major version 69" problem.

## Overview of the developement

```
                    Arch Linux
                        │
             ┌──────────┴──────────┐
             │                     │
        Tauri CLI              Android Studio
             │                     │
             │              ┌──────┴──────┐
             │              │             │
           Rust          Android SDK     NDK
             │              │             │
             └──────────────┴─────────────┘
                            │
                       Gradle / APK
                            │
                           ADB
                            │
                            ▼
                        Android Phone
```
