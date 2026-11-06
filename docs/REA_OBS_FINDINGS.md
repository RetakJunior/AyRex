# OBS incelemesi: REA bulguları ve resmi API belgeleri

İnceleme tarihi: 2026-11-06. Bu not, kurulu OBS ikilisinden elde edilen REA kanıtlarını ve OBS'nin resmi API belgelerini ayrı tutar.

## REA ile kurulu ikilide gözlenenler

REA `inspect-pe-imports`, `C:\Program Files\obs-studio\bin\64bit\obs64.exe` dosyasını doğrudan taradı. Kanıt `ev_4192a109a8756c5dd6a0e61ddc1f8820db88396be0b45c63cafca74197e0bbaa`, SHA-256 `6e2f8e9a40e221c8ec137b26dc8175b18258429ba7a0f08d6499a94befac3c17`.

- Dosya PE32+ amd64, 5,706,000 bayt.
- Import tablosu eksiksiz çözüldü: 31 DLL ve 3,651 import; delay-load import yok.
- Import adlarında `obs.dll`, `obs-frontend-api.dll`, `avcodec-62.dll`, `avformat-62.dll`, Qt 6 ve Windows grafik/sistem DLL'leri var.
- Bu, çalıştırılabilir dosyanın OBS/libobs, FFmpeg ve Qt ile yerel bir Windows uygulaması olduğunu gösterir. Import tablosu bir fonksiyonun gerçekten yüklendiğini veya çağrıldığını kanıtlamaz.

REA `inspect_desktop_application`, kurulu `C:\Program Files\obs-studio` ağacında 2,463 girdiyi inceledi ve 89 dosyayı sınırlı biçimde probeladı. Kanıt `ev_5ddf1006bd91778f22bfaa8ce4c1e4cb6d08407f977582592ec04c8af90ade2a`. Bu rapor atomik bir disk görüntüsü veya tüm dosya içeriklerinin özeti değildir; runtime'da yüklenme ya da çağrılma iddiası içermez.

Tam REA çıktıları: [PE import kanıtı](../rea-obs-imports.json) ve [kurulum ağacı incelemesi](../rea-obs-installation.json).

## REA ile WASAPI eklentisi incelemesi

Kurulum ağacında `core/win-wasapi/win-wasapi.dll` bulundu. REA `inspect-pe-imports` bu PE32+ x64 eklentiyi taradı: 86.288 bayt, SHA-256 `8512ee905e6a8d6b4d4e45f6f0e049fb79999ca5a4c9b822ff2c42bd7b06239d`; 12 DLL'den 108 import çözüldü ve import kapsamı tam raporlandı. Kanıt `ev_5e2bb83158ebcfecf9b4380fac02c1545bd39518e0e7ff886f0856c03a41e0e9` ve tam çıktı [burada](rea-obs-win-wasapi-imports.json).

Gözlenen importlar arasında `obs_register_source_s`, `obs_source_output_audio`, `obs_get_audio_info` ve kaynak özellikleri/cihaz değişikliği API'leri var. Windows tarafında `CoCreateInstance`, `CoInitializeEx`, `CoUninitialize`, `AVRT` iş parçacığı zamanlama fonksiyonları ve olay/iş parçacığı API'leri de import ediliyor. Bu statik gözlem, eklentinin OBS kaynak yaşam döngüsüne bağlandığını ve Windows ses API'lerini kullandığını gösterir; fonksiyonların hangi sırayla çalıştığını veya belirli bir çalışma anında çağrıldığını kanıtlamaz.

REA'nın Ghidra sağlayıcısıyla `obs.dll` derin analizini açma denemesi `provider_unavailable` döndürdü: bu REA sürecinde `GHIDRA_INSTALL_DIR` ayarlanmamış ve Ghidra 12.1.x bulunamadı. Ghidra kurulumu ya da REA yapılandırma değişikliği yapılmadı. Bu yüzden bu çalışmada decompile edilmiş WASAPI uygulama akışı yok; kullanılan ek kaynak, WASAPI crate'in resmi Rust belgeleri ve örnekleri.

## REA ile Windows ekran yakalama eklentisi incelemesi

REA `inspect_pe_imports`, `C:\Program Files\obs-studio\core\win-capture\win-capture.dll` dosyasını doğrudan taradı. Kanıt `ev_9affce3db8968274649bf62099bfa3eac3adf774da4c8f3250053cc1c8c392ca`; SHA-256 `1c30ed824212102a7457f22c1301d84fb2475aa0f2a90f9952fc360fafab0da0`; dosya 139.536 bayt.

- Import tablosu tam: 16 DLL ve 321 import.
- `obs.dll` imports arasında `gs_duplicator_create`, `gs_duplicator_get_texture`, `gs_duplicator_get_monitor_index`, `gs_shared_texture_available` ve `gs_texture_open_shared` bulunuyor.
- Pencere tarafında `ms_fill_window_list`, `ms_find_window`, `GetWindowRect`, `IsWindowVisible` ve `IsIconic` kullanılıyor.

Bu, modülün OBS grafik kaynağı ve Windows pencere/ekran altyapısıyla bağlarını gösteren statik import kanıtıdır. Çağrı sırası, GPU cihazı seçimi veya çalışma anındaki erişim REA import tablosundan çıkarılamaz.

## Resmi OBS belgelerinden öğrenilenler

OBS'nin [eklenti belgeleri](https://docs.obsproject.com/plugins), yerel eklentinin libobs modülü olarak yüklendiğini; `OBS_DECLARE_MODULE()` ve `obs_module_load()` kullanıldığını; kaynak, çıkış, kodlayıcı ve servislerin bu aşamada kaydedildiğini anlatır. [Modül API referansı](https://docs.obsproject.com/reference-modules) modül dışa aktarımlarını, [kaynak API referansı](https://docs.obsproject.com/reference-sources) ise `obs_register_source()` ve kaynak yaşam döngüsünü açıklar.

Bunlar belge kaynaklı API sözleşmeleridir; kurulu OBS'nin runtime davranışına dair REA kanıtı değildir.

## Windows ses API'si ve Rust bağları

Microsoft'un [WASAPI loopback açıklaması](https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording) bu modu, bir ses çıkış noktasına gönderilen akışı yakalama olarak tanımlar. Rust [`wasapi` crate 0.26 API belgesi](https://docs.rs/wasapi/0.26.0/wasapi/struct.AudioClient.html) loopback akışını `Render` uç noktasından kurup akışı `Capture` yönünde başlatmayı gösterir; crate'in [loopback örneği](https://github.com/HEnquist/wasapi-rs/blob/master/examples/loopback.rs) olay tabanlı WASAPI okumasını gösterir. Bunlar platform/library belgeleridir, OBS ikilisinden REA gözlemi değildir.

## FFmpeg ekran yakalama belgeleri

Kurulu taşınabilir FFmpeg'in `-filters` çıktısında `ddagrab` bulundu ve `-h filter=ddagrab` bunu Desktop Duplication kaynağı olarak tanımladı. FFmpeg'in [ddagrab filter belgesi](https://ffmpeg.org/ffmpeg-filters.html#ddagrab) `output_idx`, kare hızı ve ekran alanı parametrelerini açıklar. Bu API bilgisi REA bulgusu değildir. AyRex'te DXGI tarzı yakalamaya yaklaşmak için ekran kaynağına bu filter eklendi; ekran indeksi DXGI çıktısının HMONITOR tanımıyla eşleştiriliyor, eşleşmeyen ekranlarda GDI yolu kalıyor.

## AyRex için sonuç

AyRex, Rust arayüzlü bağımsız bir masaüstü uygulamasıdır; OBS modülü veya libobs ön yüzü değildir. Kurulu OBS'deki işlevler AyRex'e otomatik olarak bağlanmaz. Şu an video yakalama/kodlama için ayrı FFmpeg süreci kullanır. OBS düzeyinde çalışma için sahne kaynaklarının tek bir canlı kompozisyon ve kodlama hattında birleştirilmesi, sahne geçişlerinin aktif çıkışa uygulanması ve ayrı ses kaynaklarının gerçek zamanlı mikslenmesi gerekir.

Bu rapordaki REA gözlemleri ile API belgeleri bilinçli biçimde ayrı sunulmuştur; proje tasarımı için çıkarılan son paragraf bir mühendislik sonucudur.
