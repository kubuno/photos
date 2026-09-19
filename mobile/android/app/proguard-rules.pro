# kotlinx.serialization: keep generated serializers for the DTOs.
-keepattributes *Annotation*, InnerClasses
-dontnote kotlinx.serialization.**
-keepclassmembers class **$$serializer { *; }
-keepclasseswithmembers class com.kubuno.photos.net.** {
    kotlinx.serialization.KSerializer serializer(...);
}
-keep,includedescriptorclasses class com.kubuno.photos.net.**$$serializer { *; }
-keepclassmembers class com.kubuno.photos.net.** {
    <fields>;
}

# Retrofit: keep the API interface and its Kotlin metadata (suspend signatures).
-keep,allowobfuscation interface com.kubuno.photos.net.PhotosApi
-keepattributes Signature, Exceptions
