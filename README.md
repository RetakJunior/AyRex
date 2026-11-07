# AyRex

AyRex, Rust ile yazılmış; Windows ve Linux masaüstlerinde ekran kaydı ve yayın yapabilen bir uygulamadır. Windows sürümü kullanıcı hesabında taşınabilir ZIP olarak çalışır. Linux için AppImage hedeflenir.

## Çalışan özellikler

- Windows'ta DXGI Desktop Duplication / GDI, Linux X11 oturumunda x11grab ile ekran yakalama; canlı önizleme
- Sahne oluşturma, kopyalama, adlandırma ve silme; sahne başına ekranla birlikte birden fazla pencere, kamera, görsel, video ve metin kaynağı
- Önizlemede kaynakları sürükleyerek taşıma ve köşesinden boyutlandırma; konum, boyut ve kırpma ayarlarını kaydetme
- Önizlemede sürükleyerek sahne başına özel monitör kayıt alanı seçme
- Studio görünümünde ayrı Önizleme ve Program sahnesi seçimi; kayıt/yayın Program sahnesinden başlar
- Tek kodlama hattından çalışan Replay Buffer; kayıt sürerken son 5–600 saniyeyi ayrı klip olarak kaydeder
- Windows DirectShow ve Linux PulseAudio/PipeWire mikrofonu ile DirectShow/V4L2 kamera keşfi; kamera, görsel ve video katmanları
- Windows WASAPI veya Linux PulseAudio/PipeWire monitor kaynağından masaüstü sesi; masaüstü ve mikrofon seviyelerini ayrı ayarlama ve kayıtta ayrı ses izleri
- MKV/MP4 kayıt ve kayıtla aynı anda RTMP, RTMPS (FLV) veya SRT (MPEG-TS) çıkışı
- H.264/H.265 yazılım ve mevcut donanım kodlayıcılarını kullanma
- Ses seviyesi, gürültü azaltma, Noise Gate, Compressor, Limiter
- Parlaklık, kontrast, doygunluk, keskinlik ve kamera Chroma Key filtreleri
- Profilleri JSON dosyalarına kaydetme/yükleme
- Gerçek CPU/RAM kullanımı, FPS, dosya boyutu ve atlanan kare göstergeleri
- Windows'ta genel Ctrl+Shift+R / Ctrl+Shift+S kısayolları; Linux'ta uygulama odaktayken aynı kısayollar

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

Windows x64 taşınabilir sürüm için Releases bölümündeki `AyRex-Windows-x64.zip` dosyasını çıkarın; `AyRex.exe`, `ffmpeg.exe` ve FFmpeg DLL'leri aynı klasörde kalmalıdır. Ayar ve kayıtlar kullanıcı profilinde tutulur.

Linux x86_64 için Releases bölümündeki `AyRex-Linux-x86_64.AppImage` dosyasını indirin, çalıştırılabilir yapın (`chmod +x AyRex-Linux-x86_64.AppImage`) ve çalıştırın. Paket X11/XWayland, FFmpeg'in PulseAudio giriş desteği ve V4L2 kamera cihazları olan sistemler içindir. Wayland-only masaüstünde ekran kaydı bu sürümde desteklenmez; X11 oturumu gerekir. AppImage uygulama ve FFmpeg'i taşır, ancak Linux grafik/ses sunucularını ve ekran/kamera donanım sürücülerini sistemden kullanır; tüm dağıtımlarda ve donanımlarda çalışma garantisi verilemez. Flathub yayını ayrı paketleme ve mağaza incelemesi gerektirir.

Gizlilik için ekran önizlemesi uygulama açıldığında kapalıdır. Ekran görüntülerini yalnızca önizlemeyi kendiniz başlattığınızda alır; önizlemeyi istediğiniz zaman durdurabilirsiniz.

Flathub manifesti ve kilitlenmiş Cargo kaynakları `packaging/flatpak/` altındadır. Yerel deneme için `flatpak-builder --force-clean build-dir packaging/flatpak/io.github.retakjunior.AyRex.json` çalıştırın; Flathub'a gönderim ayrıca yapılmalıdır.

Linux geliştirme bağımlılıkları (Debian/Ubuntu):

```sh
sudo apt install build-essential pkg-config libclang-dev libxcb1-dev libxrandr-dev \
  libdbus-1-dev libpipewire-0.3-dev libwayland-dev libegl-dev libxkbcommon-dev \
  libx11-dev libxcursor-dev libxi-dev libgl1-mesa-dev libasound2-dev ffmpeg
cargo run
cargo test
cargo build --release
```

Windows geliştirme için Rust stable ve MSVC araç zinciri gerekir:

```powershell
cargo run
cargo test
cargo build --release
.\scripts\package-windows.ps1 -FfmpegPath "C:\tools\ffmpeg\bin\ffmpeg.exe" -PackageName AyRex-Windows-x64-Current
```

Windows'ta üç saniyelik gerçek masaüstü kayıt doğrulaması:

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

İlk sürümde video düzenleme/kesme veya AI işlevi yoktur. FFmpeg ayrı süreç olarak çalışır; lisans ve sürüm bilgileri `THIRD_PARTY_NOTICES.txt` ve `licenses/FFmpeg-GPLv3.txt` dosyalarındadır.
