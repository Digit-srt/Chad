# Chad

Chad, HTML/CSS/JavaScript uygulamalarını cihazda kurulu **Google Chrome** ile açan küçük bir Rust uygulama framework’ü. Windows, Linux, macOS ve Android’i hedefler. Masaüstü paketlerinde web dosyaları ve Chad çalıştırıcısı birlikte bulunur. Uygulamayı açmak için Chrome kurulumu gerekir.

## Masaüstü

Rust 1.85 veya üstü ve işletim sisteminin Rust bağlayıcısı ile CLI’yi derle.

```text
cargo build --release --locked
```

Windows PowerShell’de örnek uygulamayı paketle.

```powershell
.\target\release\Chad.exe -build .\ornek\index.html --name MerhabaChad
.\dist\MerhabaChad\MerhabaChad.exe
```

macOS ve Linux’ta aynı CLI’yi yerel olarak derleyip çalıştır.

```sh
./target/release/Chad -build ./ornek/index.html --name MerhabaChad
```

- Windows’ta paket `dist/MerhabaChad` altında oluşur. `MerhabaChad.exe`, Chad çalıştırıcısıdır. Paketi taşıyacağında klasörü bir bütün olarak taşı.
- macOS’ta `dist/MerhabaChad.app` oluşur. Chad ve web dosyaları bu uygulama paketinin içindedir. Paket yerel kullanım için geçici kimlikle imzalanır.
- Linux’ta `dist/MerhabaChad/Chad` çalıştırılabilir dosyası ve `.desktop` kısayolu oluşur. Kısayol ilk paket konumunu kullanır. Paketi taşırsan `Chad` dosyasını doğrudan çalıştırabilirsin.

`--out <paket-yolu>` çıktı konumunu değiştirir. Her derlemede yeni bir hedef yol seç. macOS’ta özel hedef yol `.app` ile bitmeli. CLI, varsayılan olarak çalıştığı işletim sistemine ve işlemci mimarisine uygun paket üretir.

## Android

Android paketi masaüstü CLI’siyle derlenir. JDK 17 ile Gradle 8.11.1’i PATH’e ekle. Android SDK’ya API 35 ve Build Tools 35.0.0’ı kur, SDK klasörünü `ANDROID_HOME` ile belirt. Windows’ta komutun başında `./target/release/Chad.exe` kullan.

```sh
./target/release/Chad -build ./ornek/index.html --name MerhabaChad \
  --target android --site https://uygulama.example.com/chad/ --app-id com.ornek.merhabachad
```

1. `--site` için kontrol ettiğin HTTPS alan adını ve web uygulamasının dizinini seç. `--app-id` Android paket kimliğini belirler.
2. Chad, `dist/MerhabaChad-android/` içine test imzalı APK ve `web/` klasörünü üretir. `web/` içeriğini HTTPS alanının köküne yayımla. Bu örnekte uygulama `/chad/index.html` ve doğrulama dosyası `/.well-known/assetlinks.json` adresinde olmalı. Doğrulama dosyasını yönlendirme olmadan `application/json` türüyle sun.
3. APK’yı Android cihazında kur ve aç. Chad, kararlı Google Chrome paketine (`com.android.chrome`) bağlanır. Chrome, uygulama imzasıyla HTTPS alanının ilişkisini doğruladıktan sonra Trusted Web Activity açılır. Doğrulanan alan içindeki uygulama sayfalarında tarayıcı araç çubuğu gizlenir. [Trusted Web Activity kurulumu](https://developer.chrome.com/docs/android/trusted-web-activity/integration-guide/)

`--project-only` Gradle projesini ve web dosyalarını SDK gerektirmeden hazırlar. Dağıtım için bu projeye kendi imzanı uygula ve sunucudaki `assetlinks.json` dosyasına dağıtılan APK’nın gerçek SHA-256 sertifika parmak izini yaz. [Android imzalama belgeleri](https://developer.android.com/studio/publish/app-signing)

## Uygulama dosyaları ve ikon

Giriş dosyan `.html` veya `.htm` olmalı. Yayınlanacak CSS, JavaScript, resim ve diğer dosyaları giriş dosyanla aynı web klasörüne koy. Chad bu klasörün içeriğini paketler. Noktayla başlayan dosya ve klasörler, sembolik bağlantılar, `dist`, `target` ve `node_modules` atlanır. Masaüstünde yerel `file://` açılışı klasik JavaScript’i destekler. ES modülleri, yerel `fetch` ve service worker gibi web sunucusu isteyen özellikler için Android’in kullandığı HTTPS dağıtımını kullan.

Varsayılan olarak Chad logosu eklenir. `--icon <dosya>` ile kendi ikonunu seçebilirsin. Windows `.ico`, macOS `.icns`, Linux ve Android `.png` kullanır. `--no-icon` işletim sistemi simgesini kullanır.
