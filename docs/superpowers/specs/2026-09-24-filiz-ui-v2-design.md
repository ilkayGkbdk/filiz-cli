# Filiz UI v2 — Tasarım Spec'i

## Amaç

Filiz’in mevcut canlı dashboard’unu, artan özellik sayısını yönetebilen bir workspace arayüzüne dönüştürmek. UI v2; daha zengin sistem metriklerini görünür kılacak, Network/Download görünümünü ayrı bir çalışma alanı olarak sunacak ve kullanıcıya görünüm, tema ve panel yoğunluğu üzerinde kontrol verecek.

## Tasarım kararları

- Ana navigation modeli üst sekmeler olacak.
- Workspace’ler numara tuşlarıyla hızlı açılacak.
- Paneller çalışma alanı içinde gizlenip gösterilebilecek.
- Mouse capture etkin olacak; wheel yalnızca odaklanmış liste/panel içinde scroll edecek.
- Ana metrik değerleri sabit görünür, ikincil ayrıntılar panel alanına göre otomatik görünür olacak.
- Tema sistemi hazır temalar ve isteğe bağlı vurgu rengi kullanacak.
- Mevcut koyu zeytin/yeşil görünüm varsayılan Forest teması olacak.

## Workspace yapısı

Üst navigation:

```text
[1 Overview] [2 Processes] [3 Network] [4 Disks] [5 More…]
```

### Overview

- Sistem sağlık durumu
- CPU, memory, disk, network kaynak kartları
- Uptime, batarya ve sıcaklık özeti
- Son olaylar ve uyarılar

### Processes

- CPU veya bellek sıralı süreç tablosu
- Metin filtresi
- Süreç detay paneli
- Terminate/kill onay modalı
- Uzun liste için mouse wheel, PageUp/PageDown ve ok tuşları

### Network

- Aktif interface seçimi
- Anlık download/upload hızı
- Oturum toplamı ve peak hız
- Zaman serisi grafikleri
- Aktif bağlantılar listesi
- Bağlantı/proses bazında trafik satırları
- Liste içinde mouse wheel scroll

### Disks

- Mount noktası seçimi
- Used/free/total kapasite
- Read/write hızı
- I/O geçmiş grafiği
- Doluluk uyarı eşiği

### More

- Sensors: sıcaklık ve batarya ayrıntıları
- Layout: panel görünürlüğü ve yoğunluk
- Theme: tema seçimi ve vurgu rengi
- About: sürüm, lisans, `for betül, with love ♡`

## Navigation ve input

- `1`–`5`: workspace seçimi
- `←` / `→`: workspace değiştirme
- `Tab`: aktif workspace içindeki panel odağını değiştirme
- `↑` / `↓`: odaklanmış liste içinde satır seçme
- `PageUp` / `PageDown`: listeyi sayfa kaydırma
- Mouse wheel: odaklanmış scrollable paneli kaydırma
- `M`: workspace menüsünü aç/kapat
- `H`: odaklanmış paneli gizle/göster
- `L`: layout yoğunluğu döngüsü: compact / balanced / spacious
- `T`: tema seçici
- `F`: aktif workspace filtresi
- `Esc`: modal, filtre veya menüyü kapatma
- `Q`: çıkış

Mouse davranışı yalnızca `EnableMouseCapture` etkin olduğunda çalışacak. Mouse hareketleri odak değiştirmeyecek; yalnızca panel içindeki wheel olayları scroll state’i değiştirecek. Terminal desteklemiyorsa keyboard navigation eksiksiz çalışmaya devam edecek.

## Metrik modeli ve görünürlük

Her kaynak kartı iki seviyeli veri gösterecek:

### CPU

- Total usage
- User/system/idle yüzdeleri
- Core count
- Per-core görünüm toggle
- Sıcaklık
- Load average

### Memory

- Used
- Free
- Available
- Cached
- Swap used/free
- Total

### Disk

- Used
- Free
- Total
- Read/write bytes per second
- Mount noktası
- Doluluk yüzdesi

### Network

- Download/upload anlık hız
- Download/upload session total
- Peak hız
- Aktif interface
- Aktif bağlantı sayısı

Bir değer desteklenmiyorsa `N/A` gösterilecek. Eksik örnekler geçmiş grafiklere `0` olarak yazılmayacak; grafik ya boş bırakılacak ya da son geçerli örnek korunacak.

## Layout sistemi

Her workspace panel düzenini `LayoutState` ile yönetecek. Panel görünürlüğü, seçilen yoğunluk ve panel sırası UI state içinde tutulacak.

- `compact`: terminal yüksekliği düşükken ana değerler ve kritik tablolar
- `balanced`: varsayılan; ana değerler ve seçili ikincil ayrıntılar
- `spacious`: daha büyük grafikler ve daha fazla satır

Gizlenen paneller tamamen veri toplamayı durdurmayacak; yalnızca render’dan çıkarılacak. Böylece panel yeniden açıldığında geçmiş grafik aniden boşalmayacak.

## Tema sistemi

Hazır temalar:

- `Forest`: koyu zeytin arka plan, lime/yeşil vurgu
- `Amber`: koyu kömür arka plan, amber vurgu
- `Mono`: gri tonları, yalnızca durumlar için kontrast
- `Solarized`: düşük göz yorgunluğu için solarized dark tonları

Her tema background, panel, border, text, muted, normal, warning ve danger renklerini tanımlayacak. Kullanıcı ayrıca vurgu rengini preset içinden seçebilecek. Tema seçimi ilk etapta sadece runtime state olacak; kalıcı config dosyası sonraki küçük pakette eklenebilir.

## Network/Download veri akışı

Network collector mevcut byte-rate ölçümlerini koruyacak ve aşağıdaki snapshot alanlarını genişletecek:

- interface totals
- current rates
- peak rates
- session totals
- active connection summary

Bağlantı listesi ayrı bir scroll state taşıyacak. Ağ verisi bulunamadığında panel çökmeden `N/A` ve kısa collector uyarısı gösterecek. İlk snapshot’ta rate değerleri `N/A` olabilir; sahte sıfır gösterilmeyecek.

## Installer ve splash ilişkisi

Mevcut ANSI logo splash korunacak. UI v2’nin `About` workspace’inde aynı logo metin/ANSI fallback ile gösterilecek. Release installer ayrı bir script/asset olarak ilerleyecek; `cargo install` geliştirici kurulumu olarak kalacak.

## Hata ve erişilebilirlik davranışı

- Küçük terminalde içerik kırpılmayacak; kritik değerler önceliklendirilecek.
- Aksiyon uyarıları ve collector hataları compact layout’ta görünür kalacak.
- Mouse çalışmıyorsa hiçbir temel navigasyon kilitlenmeyecek.
- Tema kontrastı metin ve durum renklerini okunabilir tutacak.
- Modal açıldığında mouse wheel arka paneli kaydırmayacak.

## Test kapsamı

- Workspace geçişi ve numara kısayolları
- Panel focus/gizleme/gösterme
- Layout density döngüsü
- Tema state ve renk paleti seçimi
- Mouse wheel’in yalnızca odak paneli scroll etmesi
- PageUp/PageDown ve keyboard fallback
- Metrik `N/A` davranışı
- Network rate/total/peak dönüşümleri
- Network listesi scroll state’i
- Her workspace’in normal ve compact terminal boyutlarında render edilmesi
- `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo build --release`

## Aşamalı teslimat

1. Workspace navigation, focus, hide/show, density ve mouse scroll altyapısı.
2. Tema sistemi ve ayrıntılı CPU/memory/disk kartları.
3. Network/Download workspace’i ve bağlantı listesi.
4. More workspace’i, About görünümü ve layout/theme menüleri.
5. UI v2 testleri, README güncellemesi ve GitHub Actions doğrulaması.
