# AyRex

AyRex, Windows için geliştirilmiş hafif bir masaüstü ekran kaydedicisidir.
Ekranı gerçekten yakalar; H.264/H.265 videoyu MKV veya MP4 olarak kaydeder.
Arayüz Rust ve egui, ekran yakalama XCap, kodlama ise FFmpeg kullanır.

## Kullanım

Windows x64 için **AyRex Taşınabilir** ZIP paketini indirin, tamamını aynı
klasöre çıkarın ve `AyRex.exe` dosyasını açın. Kurulum sihirbazı veya ayrıca
FFmpeg yüklemeniz gerekmez: kayıt motoru pakete dahildir. `AyRex.exe` ile
`ffmpeg.exe` aynı klasörde kalmalıdır.

Uygulama açıldığında monitör önizlemesi görünür. Sağ panelden kayıt çözünürlüğü,
kare hızı, FFmpeg kodlayıcıları, bitrate, dosya biçimi ve kayıt klasörü
seçilir. İsteğe bağlı bir DirectShow ses aygıtı seçilip ses düzeyi ayarlanabilir
ve kayıt öncesinde ses girdisi sessize alınabilir. **Start recording** kaydı
başlatır; **Stop recording** dosyayı düzgün biçimde tamamlar. Sonuçta kayıt
klasörünü uygulamanın içinden açabilirsiniz.

Ayarlar uygulama kapatılırken kaydedilir. Varsayılan kayıt klasörü
`%USERPROFILE%\Videos\AyRex` dizinidir.

## Geliştiriciler

Rust stable ve Windows SDK/MSVC araçları gereklidir:

```powershell
cargo run
cargo test
cargo build --release
.\scripts\package-windows.ps1 -FfmpegPath "C:\tools\ffmpeg\bin\ffmpeg.exe"
```

Paketleme betiği `dist\AyRex-Windows-x64.zip` üretir. `-FfmpegPath`, H.264
(`libx264`) kodlayıcılı ve x64 Windows için statik derlenmiş taşınabilir FFmpeg'i
göstermelidir.

Komut satırından gerçek masaüstünde 30 saniyelik doğrulama kaydı almak için:

```powershell
cargo run -- --smoke-record 30
```

Test kaydı `%TEMP%\AyRexSmokeTest` dizinine yazılır. FFmpeg başka bir konumdaysa
`AYREX_FFMPEG` ortam değişkeniyle dosya yolunu belirtebilirsiniz.

## Mimari

- `src/app.rs`, `src/state.rs`, `src/config.rs`: uygulama, çalışma durumu ve
  kalıcı kayıt seçenekleri.
- `src/capture/`: monitör keşfi ve asenkron gerçek ekran önizlemesi.
- `src/recording/`: FFmpeg kodlayıcı ayarları, kayıt süreci, ilerleme ve çıktı.
- `src/audio/`: DirectShow ses aygıtlarının gerçek keşfi.
- `src/system/`: FFmpeg ve kullanılabilir kodlayıcıların keşfi.
- `src/ui/`: önizleme, kayıt ayarları, canlı durum ve kayıt kontrolleri.
- `build.rs`: PNG logodan Windows EXE simgesi üretimi.

## Sınırlar

Bu sürüm monitörü kaydeder; sahne birleştirme, pencere/bölge yakalama, sistem
sesini loopback üzerinden alma ve canlı yayın henüz uygulanmamıştır. Ses kaydı,
yalnızca FFmpeg'nin bulduğu seçili DirectShow ses aygıtı içindir. Kodlayıcılar
donanıma ve FFmpeg paketine göre değişir; donanım kodlayıcısı başlatılamazsa
uygulama H.264 yazılım kodlayıcısına geçer.

## Üçüncü taraf yazılım

Taşınabilir Windows paketindeki FFmpeg, GPL özellikli Gyan.Dev statik
derlemesidir. Lisans, sürüm ve kaynak koduna erişim bilgileri paketteki
`THIRD_PARTY_NOTICES.txt` ve `FFmpeg-GPLv3.txt` dosyalarındadır. FFmpeg,
AyRex'e kütüphane olarak bağlanmaz; ayrı bir süreç olarak çalışır.
