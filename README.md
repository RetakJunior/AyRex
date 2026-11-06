# AyRex

AyRex, Rust ile yazılmış ve Windows'ta kullanıcı hesabı altında çalışan masaüstü kayıt/yayın uygulamasıdır. Kurulum sihirbazı ya da yönetici izni gerektirmez; taşınabilir ZIP'i kullanıcı tarafından yazılabilir bir klasöre çıkarmak yeterlidir.

## Çalışan özellikler

- DXGI Desktop Duplication ekran yakalama; uygun olmayan monitörlerde GDI uyumluluk yolu; açık pencere katmanları ve canlı önizleme
- Sahne oluşturma, kopyalama, adlandırma ve silme; sahne başına ekranla birlikte birden fazla pencere, kamera, görsel, video ve metin kaynağı
- Önizlemede kaynakları sürükleyerek taşıma ve köşesinden boyutlandırma; konum, boyut ve kırpma ayarlarını kaydetme
- Önizlemede sürükleyerek sahne başına özel monitör kayıt alanı seçme
- Studio görünümünde ayrı Önizleme ve Program sahnesi seçimi; kayıt/yayın Program sahnesinden başlar
- Tek kodlama hattından çalışan Replay Buffer; kayıt sürerken son 5–600 saniyeyi ayrı klip olarak kaydeder
- DirectShow mikrofon ve kamera keşfi; kamera, görsel ve video katmanlarını kompozisyona bindirme
- Varsayılan Windows ses çıkışını WASAPI loopback ile yakalama; masaüstü ve mikrofon seviyesini ayrı ayarlama ve kayıtta ayrı ses izleri
- MKV/MP4 kayıt ve kayıtla aynı anda RTMP, RTMPS (FLV) veya SRT (MPEG-TS) çıkışı
- H.264/H.265 yazılım ve mevcut donanım kodlayıcılarını kullanma
- Ses seviyesi, gürültü azaltma, Noise Gate, Compressor, Limiter
- Parlaklık, kontrast, doygunluk, keskinlik ve kamera Chroma Key filtreleri
- Profilleri JSON dosyalarına kaydetme/yükleme
- Gerçek CPU/RAM kullanımı, FPS, dosya boyutu ve atlanan kare göstergeleri
- AyRex odakta olmasa da Ctrl+Shift+R ile başlatma ve Ctrl+Shift+S ile durdurma

## Kick / Twitch yayını

`Kayıt ve Yayın` bölümünde **Yayın yap** seçeneğini aç, Kick veya Twitch'i
seçip platform panelindeki yayın anahtarını gir ve **Kaydı Başlat** düğmesine
bas. Kayıt ve yayın aynı H.264 kodlama hattını paylaşır. Kick ve Twitch için
RTMPS sunucuları hazır gelir; Kick panelinden farklı bir sunucu
verildiyse Sunucu adresi alanına yapıştır. Sunucu kök adresi `/app` ile
bitmiyorsa AyRex bunu ekler. Yayın anahtarı uygulama ayarlarına veya profillere
kaydedilmez; uygulama kapatılınca yeniden girilmesi gerekir.

Kick en fazla 8.000 kbps; Twitch için bu arayüz 6.000 kbps ile sınırlar.
Her iki platformda da H.264 kodlayıcı kullan. Yayın bağlantısı kurulamazsa
yerel kayıt korunur ve bağlantı hatası kayıt sonunda gösterilir.
Anahtarı kimseyle paylaşma; ifşa olduğunu düşünüyorsan platform panelinden
yenile.

Kick yayın bilgileri: https://dashboard.kick.com/stream

Twitch yayın anahtarı: https://dashboard.twitch.tv/settings/stream

Twitch RTMP adres biçimi: https://dev.twitch.tv/docs/video-broadcast/

## Çalıştırma

Windows x64 taşınabilir sürüm için `dist/AyRex-Windows-x64-Current.zip` dosyasını çıkarın; `AyRex.exe` ile `ffmpeg.exe` aynı klasörde kalmalıdır. Ayar ve kayıtlar kullanıcı profilinde tutulur. Geliştirme için Rust stable ve MSVC araç zinciri gerekir:

```powershell
cargo run
cargo test
cargo build --release
.\scripts\package-windows.ps1 -FfmpegPath "C:\tools\ffmpeg\bin\ffmpeg.exe" -PackageName AyRex-Windows-x64-Current
```

Üç saniyelik gerçek masaüstü kayıt doğrulaması:

```powershell
.\target\release\ayrex.exe --smoke-record 3
.\target\release\ayrex.exe --smoke-region 3
.\target\release\ayrex.exe --smoke-system-audio 3
.\target\release\ayrex.exe --smoke-replay 5
.\target\release\ayrex.exe --check-hotkeys
```

## Sürüm durumu

AyRex bağımsız Rust uygulamasıdır; OBS eklentisi değildir. OBS'in yerini bütünüyle alması için çalışma sürüyor. REA'nın kurulu OBS `win-capture` modülünde gözlediği `gs_duplicator_*` ekran kaynak API'lerine yaklaşmak için uygun monitörlerde DXGI Desktop Duplication yakalama kullanılır; bu, OBS ile aynı libobs kaynağı değildir. Studio görünümü Program sahnesini kayıt/yayın başlatılırken kullanır; canlı yayın sürerken sahne kesme/geçiş henüz eklenmedi. Kayıt ve Replay Buffer mikrofonla masaüstünü ayrı ses izlerine yazar; canlı yayın bunları tek miks olarak gönderir. Yerleşik tarayıcı kaynağı, sanal kamera, multiview ve projector henüz tamamlanmadı. Aynı kamera cihazı bir sahneye bir kez eklenebilir.

Kurulu OBS için REA bulguları ile resmi OBS API belgelerinden çıkan bilgilerin ayrımı [inceleme notunda](docs/REA_OBS_FINDINGS.md) tutulur.

İlk sürümde video düzenleme/kesme veya AI işlevi yoktur. FFmpeg taşınabilir paketle ayrı süreç olarak çalışır; lisans ve sürüm bilgileri `THIRD_PARTY_NOTICES.txt` ve `FFmpeg-GPLv3.txt` dosyalarındadır.
