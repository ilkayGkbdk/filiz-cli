# Filiz Temel Altyapı (Foundation) — Tasarım Spec'i

## Amaç

Filiz'in yeni arayüz, efektif menüler, özelleştirilebilir tema, interaktif butonlar ve terminal UX iyileştirmeleri hedeflerine ulaşabilmesi için gereken altyapıyı kurmak. Bu alt proje görünümü değiştirmez; mimariyi yeniden düzenler ve UI v2'de tespit edilen hataları düzeltir.

Bu spec, sonraki alt projelerin (release tabanlı installer, mock'lara uygun workspace görselleri, command palette/menü, tema seçici ve kalıcı config) ön koşuludur. O alt projeler ayrı spec ve plan döngüleriyle ele alınacaktır.

## Kullanıcı niyeti ve başarı ölçütleri

- Arayüz hiçbir zaman donmaz; veri toplama ne kadar sürerse sürsün tuşlar en geç 50 ms içinde işlenir.
- Tuş çakışmaları yapısal olarak engellenir; aynı bağlamda aynı tuşun iki kez tanımlanması testte yakalanır.
- Yeni workspace, panel veya buton eklemek tek bir dosya ve bir kayıt satırıyla mümkündür.
- Testler davranışı doğrular; yalnızca "ekranda etiket var mı" kontrolü yapan testler davranış testleriyle değiştirilir.
- Kapsam kararı: görünüm korunur, tespit edilen hatalar düzeltilir. Tek görünür yapısal değişiklik, Overview'un kopyası olan Processes workspace'inin kendi düzenine kavuşmasıdır.

## Kapsam dışı

- Yeni görsel tasarım (dashboard-revision mock'undaki yan yana düzen, disk bar'ları vb.)
- Command palette içeriği; bu projede yalnızca `M` tuşu menü giriş noktası olarak ayrılır
- Tema seçici, vurgu rengi seçimi ve kalıcı config dosyası
- Release pipeline ve installer yeniden yazımı
- Disk read/write ölçümü, per-core CPU, memory cached

## Mimari

```text
collector worker thread'leri ──CollectorUpdate──▶ mpsc kanal
                                                    │
                           ana döngü: drain → SystemState::apply → History
                                                    │
          crossterm Event → Keymap/HitMap → Action → App::update → Effect
                                                    │
                           ui::render(&App) → Frame + HitMap
```

Eşzamanlılık standart kütüphane thread'leri ve `std::sync::mpsc` ile sağlanır. Yeni runtime bağımlılığı (tokio vb.) eklenmez.

## 1. Veri modeli

### SystemState

`Vec<ResourceMetric>` ve string isimli metrik aramaları (`"disk./.usage"`) kaldırılır. Yerine typed durum gelir:

```rust
pub struct SystemState {
    pub cpu: CpuStats,                 // usage, user, system, idle, cores, load: [f64; 3], temperature
    pub memory: MemoryStats,           // total, used, available, free, swap_used, swap_total
    pub disks: Vec<DiskStats>,         // mount, total, used, free, is_system
    pub interfaces: Vec<InterfaceStats>, // name, rx_rate, tx_rate, rx_total, tx_total, peak_rx, peak_tx
    pub processes: Vec<ProcessInfo>,   // mevcut alanlar + traffic: Option<TrafficRate>
    pub battery: Option<Battery>,      // percent, charging, power_source
    pub uptime: Option<Duration>,
    pub issues: SourceIssues,          // kaynak bazında hata kaydı
}
```

- Ölçülemeyen her değer `Option`'dır ve UI'da `N/A` olarak gösterilir.
- `is_system`, `/System/Volumes/` altındaki ve `/private/var/vm` gibi sistem mount'ları için `true` olur; Disks listesi varsayılan olarak bunları gizler.

### CollectorUpdate ve apply

```rust
pub enum Source { System, Platform, Traffic }

pub enum CollectorUpdate {
    System(SystemSample),        // cpu usage/idle/cores/load, memory, disks, interfaces, processes, uptime
    Platform(PlatformSample),    // cpu user/system, battery, temperature
    Traffic(Vec<ProcessTraffic>),// pid, rx byte/sn, tx byte/sn (nettop yalnızca pid verir)
    Failed { source: Source, message: String },
}
```

`SystemState::apply(&mut self, update: CollectorUpdate)` saf fonksiyondur:

- Yalnızca update'in sahip olduğu alanları yazar; diğer kaynakların verisine dokunmaz.
- Başarılı bir update, o kaynağın `issues` kaydını temizler.
- `Failed` update, o kaynağın alanlarını `None`'a çeker ve `issues` kaydını yazar; bayat değer güncelmiş gibi gösterilmez.
- `Traffic` update, süreç listesindeki eşleşen pid'lere `traffic` alanını yazar; eşleşmeyen süreçlerde `None` kalır.

### History

Grafik geçmişi `App.histories: [Vec<u64>; 4]` yerine `History` tipine taşınır:

```rust
pub enum SeriesKey { Cpu, Memory, Disk, NetRx(String), NetTx(String) }
pub struct History { series: HashMap<SeriesKey, VecDeque<u64>>, capacity: usize } // capacity 60
```

- Değeri `None` olan örnek geçmişe yazılmaz (sahte sıfır yok).
- Download ve upload grafikleri seçili interface'in `NetRx`/`NetTx` serisini çizer.
- Artık görünmeyen interface serileri bir sonraki `System` update'inde silinir.

### Sağlık durumu

- "Desteklenmiyor" (ör. sıcaklık sensörü yok) `N/A` olarak gösterilir, uyarı sayılmaz.
- Status bandı yalnızca `issues` içinde bir kaynak hatası varsa uyarı rengine döner.

## 2. Collector runtime

`collectors/runtime.rs` içinde `spawn(tx: Sender<CollectorUpdate>) -> CollectorRuntime`:

| Worker | Aralık | Kaynak |
|---|---|---|
| `system` | 2 sn | sysinfo: CPU, bellek, disk, ağ interface'leri, süreç listesi |
| `platform` | 5 sn | `/usr/bin/top -l 1 -n 0` (user/sys), `/usr/bin/pmset -g batt`, sysinfo Components |
| `traffic` | akış | Sürekli açık `nettop -P -x -L 0 -s 2` alt süreci |

- Her worker kendi kontrol kanalında `recv_timeout(interval)` ile bekler. Kontrol mesajları: `Refresh` (anında topla) ve `Stop`.
- `CollectorRuntime::refresh()` tüm worker'lara `Refresh` gönderir (`R` tuşu).
- `CollectorRuntime` drop edildiğinde `Stop` gönderilir, `nettop` alt süreci öldürülür ve thread'ler join edilir.
- Mevcut `Collector` trait'i ve hesaplama fonksiyonları (`byte_rate`, `percentage`, `parse_cpu_usage`, `parse_battery`) korunur; worker'lar bunları sarar.

### Traffic worker

- `nettop` stdout'u satır satır okunur. `time,` ile başlayan satır yeni bir blok başlatır; header sütun adları blok başlığından okunur.
- İkinci sütun `ad.pid` biçimindedir; son `.` sonrası pid olarak ayrıştırılır.
- `bytes_in` ve `bytes_out` kümülatiftir. İki ardışık blok arasındaki fark, bloklar arası süreye bölünerek byte/sn üretilir.
- Sayaç azalırsa (süreç yeniden başladı) o pid için rate üretilmez ve baz sıfırlanır.
- Ayrıştırma ve fark hesabı saf fonksiyonlardır (`NettopParser::push_line`, `TrafficTracker::update`).
- `nettop` başlatılamazsa veya kapanırsa `Failed { source: Traffic }` gönderilir. Worker 10 sn aralıkla en fazla 3 kez yeniden dener; sonra durur.
- Trafik hatası yalnızca Network workspace'inde uyarı olarak gösterilir.

### Hata izolasyonu

- Worker içindeki panic yalnızca o thread'i düşürür.
- Ana döngü kanal bağlantısının koptuğunu (`TryRecvError::Disconnected`) veya bir worker'ın bittiğini algılarsa status bandında "veri akışı durdu" uyarısı gösterir; uygulama kapanmaz.

## 3. Ana döngü ve girdi

### Döngü

```text
loop:
  1. rx.try_recv() ile tüm update'leri al → state.apply + history → dirty
  2. event::poll(≤ 50 ms) → Key / Mouse / Resize → Action → app.update(action) → Vec<Effect>
  3. Effect: Quit | Refresh | SendSignal(ConfirmedAction)
  4. dirty || app.animating() → terminal.draw; oluşan HitMap app'e kaydedilir
```

- `App::update(&mut self, action: Action) -> Vec<Effect>` işletim sistemiyle konuşmaz; sinyal gönderme ve runtime yenileme effect olarak döngüde yürütülür.
- `app.animating()` bu projede spinner ve splash için kullanılır; sonraki alt projeler geçiş animasyonları için bu tick'i kullanır.
- Boştayken ekran yeniden çizilmez.

### Splash

- Logo satırları `\r\n` ile basılır (raw mode'da merdiven kayması giderilir).
- Splash en fazla 650 ms görünür, herhangi bir tuş atlatır ve runtime bu sırada zaten veri toplamaya başlamıştır.

### Keymap

`input/keymap.rs`:

```rust
pub enum Context { Confirm, Detail, Filter, Menu, Panel(PanelId), Workspace(Workspace), Global }
pub struct Binding { context: Context, key: KeySpec, action: Action, hint: Option<&'static str> }
pub static BINDINGS: &[Binding] = &[ /* tek tablo */ ];
```

Bağlam yığını öncelik sırasıyla:

```text
Confirm (exclusive) > Detail > Filter (exclusive) > Menu > Panel(odaktaki) > Workspace(ws) > Global
```

- `Panel(PanelId)` bağlamı odaktaki panelin bağlamalarıdır; örneğin süreç tuşları hem Overview hem Processes workspace'inde süreç paneli odaktayken çalışır.
- İlk eşleşen bağlam kazanır. Exclusive bağlamlar tabloda olmayan tuşları yutar; Filter bağlamında yazdırılabilir karakterler `FilterInput(char)` aksiyonuna çevrilir.
- Footer ipuçları aynı tablodan, aktif bağlam yığınına göre üretilir.

Tuş kararları:

| Tuş | Bağlam | Aksiyon |
|---|---|---|
| `Q` | Global | Quit |
| `1`–`5`, `←`/`→` | Global | Workspace seç / değiştir |
| `Tab` / `Shift+Tab` | Global | Aktif workspace'in görünür panelleri arasında odak |
| `↑`/`↓`, `PageUp`/`PageDown`, `Home`/`End` | Global | Odaktaki listede hareket |
| `Enter` | Processes paneli | Detay |
| `Esc` | Global | Kapat / filtreyi temizle |
| `F` | Processes paneli | Filtre |
| `S` | Processes paneli | Sıralama döngüsü CPU → MEM |
| `K` / `Shift+K` | Processes paneli / Detail | Terminate / Kill onayı |
| `Y` / `N` | Confirm | Onayla / iptal |
| `M` | Global | Menü aç/kapat (giriş noktası) |
| `H` / `L` / `T` | Global | Panel gizle / yoğunluk / tema döngüsü |
| `R` | Global | Anında yenile |

- `C`/`M` ile sıralama kaldırılır; aktif sıralama sütun başlığında `▼` ile gösterilir.
- Network workspace'inde `↑`/`↓` gizli davranış taşımaz; interface listesi odaktaysa interface seçer.

### ListState

```rust
pub struct ListState { pub selected: usize, pub offset: usize }
impl ListState {
    fn clamp(&mut self, len: usize, viewport: usize);
    fn move_by(&mut self, delta: isize, len: usize, viewport: usize);
}
```

- Tüm listeler (süreçler, interface'ler, bağlantılar, diskler) `ListState` kullanır; değerler her zaman `len` ile sınırlanır.
- PageUp/PageDown son render'daki görünür satır sayısı kadar hareket eder.
- Süreç seçimi `ProcessIdentity` ile snapshot'lar arasında korunur (mevcut davranış).

## 4. UI yapısı

```text
src/ui/
  mod.rs          render(frame, &App) -> HitMap; üst bant, sekmeler, footer, workspace yönlendirme
  format.rs       bytes, rate, percent, uptime
  theme.rs        rol token'lı Palette ve dört tema
  hit.rs          HitMap, HitTarget
  components/     status.rs, tabs.rs, card.rs, list.rs, modal.rs, footer.rs
  workspaces/     overview.rs, processes.rs, network.rs, disks.rs, more.rs
```

```rust
pub trait WorkspaceView {
    fn panels(&self) -> &'static [PanelId];
    fn render(&self, frame: &mut Frame, area: Rect, cx: &mut RenderCx);
}
pub struct RenderCx<'a> { pub app: &'a App, pub palette: Palette, pub focus: PanelId, pub hits: &'a mut HitMap }
```

- `widgets.rs` bu yapıya bölünerek kaldırılır; `app.rs` ve `widgets.rs` içindeki kopya yardımcılar `format.rs`'e taşınır.
- Overview, Network, Disks ve More mevcut görünümlerini korur.
- Processes: tam yükseklikte süreç tablosu ve altta detay şeridi (mevcut bileşenlerle).
- `H` ve `L` her workspace'te o workspace'in panellerine uygulanır.

### HitMap

```rust
pub enum HitTarget { Tab(Workspace), Panel(PanelId), Row(PanelId, usize), Button(Action), Blocker }
pub struct HitMap { entries: Vec<(Rect, HitTarget)> }  // sonra eklenen üstte
```

- Sol tık: sekme → workspace; panel → odak; satır → odak + seçim; aynı satıra ikinci tık → `Open`; buton → aksiyonu.
- Wheel: imlecin altındaki paneli kaydırır; imleç panel üstünde değilse odaktaki paneli.
- Footer ipuçları `Button(Action)` olarak kaydedilir.
- Modal açıkken tam ekran `Blocker` kaydedilir; modal dışındaki tıklama ve wheel yok sayılır.

### Tema token'ları

```rust
pub struct Palette {
    pub bg: Color, pub surface: Color, pub surface_alt: Color,
    pub border: Color, pub border_focus: Color,
    pub text: Color, pub text_muted: Color,
    pub accent: Color, pub accent_fg: Color,
    pub selection_bg: Color, pub selection_fg: Color,
    pub ok: Color, pub warn: Color, pub danger: Color, pub info: Color,
    pub chart_rx: Color, pub chart_tx: Color,
    pub danger_modifier: Modifier,
}
```

- Forest, Amber, Mono, Solarized bu token'lara eşlenir; Forest mevcut renkleri birebir korur.
- Mono'da `danger_modifier = REVERSED | BOLD`.
- `theme.rs` içindeki eski sabitler (`BACKGROUND`, `PANEL` …) silinir; UI kodu yalnızca `Palette` kullanır.

## 5. Hata yönetimi

- Collector hatası: ilgili alanlar `N/A`, uyarı ilgili panelde, kaynak düzelince uyarı kendiliğinden temizlenir.
- Worker ölümü: "veri akışı durdu" uyarısı, uygulama çalışmaya devam eder.
- Terminal: `TerminalGuard` korunur; ek olarak panic hook terminali restore edip ardından mesajı basar.
- Süreç aksiyonları: açık onay, kendini hedefleme koruması ve `ProcessIdentity` doğrulaması korunur.

## 6. Düzeltilecek hatalar

1. `M` tuşu hem menü hem bellek sıralaması; sıralama `S`'e taşınır.
2. Network interface indeksinin sınırsız artması.
3. Bağlantı listesinde sınırsız scroll offset ve kaydırılmış liste uzunluğunu gösteren başlık.
4. PageUp/PageDown'ın süreç tablosunda etkisiz olması.
5. `nettop -J` geçersiz bayrağı nedeniyle süreç trafiğinin hiç gelmemesi.
6. Download ve upload grafiklerinin aynı toplam veriyi çizmesi.
7. Processes workspace'inin Overview'un kopyası olması.
8. `Tab` odağının görünmeyen panellere gitmesi; `H`/`L`'nin yalnızca Overview'da çalışması.
9. Footer'ın her workspace'te süreç tuşlarını göstermesi.
10. Splash'in raw mode'da merdiven kayması ve bloklayan beklemesi.
11. Sensör yokluğunda status bandının kalıcı uyarı rengi; sistem volume'larının Disks listesini doldurması.
12. Veri toplamanın UI thread'inde yapılması nedeniyle her yenilemede donma.

## 7. Test stratejisi

- Saf mantık: `SystemState::apply` (kaynak izolasyonu, `Failed` sonrası `None`, issue temizleme), `History` (kapasite, `None` atlama, seri temizleme), `NettopParser` ve `TrafficTracker` (blok ayrıştırma, `ad.pid`, fark/saniye, sayaç sıfırlanması), `ListState` sınırları, keymap çözümlemesi (öncelik, exclusive bağlam, tablo çakışma testi), `App::update` effect'leri, HitMap tıklama → aksiyon çevirisi.
- Render: `TestBackend` ile hücre düzeyinde doğrulama — seçili satır vurgusu, bağlama göre footer, aktif sekme, modal açıkken arka plan tıklamasının yok sayılması, her workspace'in 110×35, 80×24, 48×20 ve 20×8 boyutlarında panic'siz çizilmesi.
- Runtime: sahte collector'larla; 1 sn uyuyan bir collector çalışırken girdi aksiyonlarının beklemeden işlendiği ve `Stop` sonrası thread'lerin join edildiği doğrulanır.
- Zorunlu kontroller değişmez: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo build --release`.

## 8. Aşamalı teslimat

Her adım testler yeşil olarak ayrı commit'lenir:

1. `ui/format.rs` ve tema token'ları (davranış değişmez).
2. Typed `SystemState`, `CollectorUpdate`, `apply`, `History`; mevcut collector'lar adapte edilir.
3. Collector runtime thread'leri ve yeni ana döngü (hata 12).
4. Keymap, `Action`/`App::update`/`Effect`, `ListState` (hata 1–4, 8, 9).
5. `components/` ve `workspaces/` ayrımı, Processes düzeni (hata 6, 7).
6. HitMap ve mouse tıklamaları.
7. Nettop akış worker'ı (hata 5), splash (hata 10), sensör/volume düzeltmeleri (hata 11).
8. README tuş tablosu ve CHANGELOG güncellemesi.
