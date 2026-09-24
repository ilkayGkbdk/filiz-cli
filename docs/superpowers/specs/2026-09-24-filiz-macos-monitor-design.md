# Filiz macOS System Monitor — Tasarım Spec'i

## Amaç

Filiz, macOS üzerinde `filiz` komutuyla çalışan, bilgisayarın canlı durumunu anlaşılır bir terminal arayüzünde gösteren ve seçili süreçler üzerinde güvenli aksiyonlar alabilen bir sistem izleme aracıdır.

İlk sürümün başarısı, kullanıcının dashboard'a baktığında birkaç saniye içinde sistemin genel durumunu anlayabilmesi ve gerektiğinde yüksek kaynak kullanan bir süreci kontrollü biçimde yönetebilmesiyle ölçülür.

## Kapsam

### İlk sürümde olacaklar

- CPU kullanımı, çekirdek kullanımı ve sıcaklık
- RAM ve swap kullanımı
- Disk kapasitesi ve temel I/O bilgisi
- Ağ indirme/yükleme trafiği
- Batarya yüzdesi, şarj durumu ve tahmini durum
- Süreç listesi; CPU ve bellek sıralaması
- Süreç detay görünümü
- Süreç sonlandırma ve kill aksiyonu için onay akışı
- Süreç filtreleme ve sıralama
- Canlı yenileme
- Klavye ile panel ve liste navigasyonu
- macOS üzerinde kolay kurulum ve `filiz` komutunun PATH üzerinden çalışması

### İlk sürüm dışında tutulacaklar

- Linux veya Windows desteği
- Sistem ayarlarını değiştiren işlemler
- Disk temizleme ve servis yönetimi
- Uzaktan izleme
- Kalıcı geçmiş/veritabanı
- Web arayüzü

## Kullanıcı deneyimi

Dashboard dört seviyeli bir bilgi hiyerarşisine sahip olacak:

1. Üst durum satırı: Sistem normal mi, uyarı var mı, uptime, batarya ve sıcaklık.
2. Kaynaklar: CPU, bellek, disk ve ağ için büyük değerler ve kısa canlı grafikler.
3. Süreçler: En çok kaynak kullanan süreçler ve klavye aksiyonları.
4. Detay: Disk özeti ve son olaylar.

Arayüz koyu zeytin/siyah bir terminal paleti, yüksek kontrastlı metin ve sınırlı vurgu renkleri kullanacak. Renkler anlam taşıyacak: yeşil normal, sarı dikkat, kırmızı tehlike. Bilgi yoğunluğu btop seviyesinde tutulacak fakat ilk bakıştaki hiyerarşi daha belirgin olacak.

Temel kısayollar:

- `Tab`: panel değiştir
- `↑` / `↓`: liste içinde gezin
- `Enter`: süreç detayını aç
- `K`: süreç sonlandırma akışını başlat
- `F`: filtrele
- `Q`: çıkış

Her yıkıcı aksiyon, hedef süreç adı, PID ve seçilen aksiyonu gösteren açık bir onay ekranı gerektirir. Başarılı veya başarısız aksiyon sonucu kısa süreli olay mesajı olarak görünür.

## Mimari

```text
macOS sistem kaynakları
        ↓
Collector katmanı
        ↓
SystemSnapshot modeli
        ↓
App state / input handling
        ↓
Ratatui UI
```

### Collector katmanı

Collector'lar tek bir ortak uygulama durumuna bağımlı olmayacak. Her collector kendi sorumluluğundaki veriyi üretir ve mümkün olduğunca ölçüm hatasını lokal olarak raporlar.

- `cpu`: kullanım, çekirdekler, yük ve sıcaklık
- `memory`: toplam, kullanılan, boş, cached ve swap
- `disk`: mount noktaları, kapasite ve I/O
- `network`: arayüz bazında byte/saniye değerleri
- `battery`: yüzde, güç kaynağı ve şarj durumu
- `process`: süreç listesi ve süreç detayları

İlk uygulama için genel sistem ölçümlerinde `sysinfo` kullanılacak. macOS'a özel sıcaklık, batarya ve gerektiğinde daha hassas süreç bilgileri ayrı bir platform modülünde tutulacak. Platform API'si bulunamadığında ilgili alan `N/A` olarak gösterilecek; tek bir eksik sensör dashboard'ın tamamını durdurmayacak.

### State modeli

Collector sonuçları render katmanına doğrudan aktarılmayacak. Her yenileme turunda bir `SystemSnapshot` üretilecek. UI yalnızca son snapshot'ı okuyacak. Bu, veri toplama ile çizim zamanlamasını ayırır ve test edilebilirliği artırır.

Snapshot içinde ölçüm zamanı, sistem özeti, kaynak metrikleri, süreç listesi ve varsa collector uyarıları bulunacak.

### Uygulama ve aksiyonlar

Uygulama state'i seçili paneli, seçili süreci, filtreyi, sıralamayı ve modal/onay durumunu tutacak. Render saf bir dönüşüm olarak kalacak. Süreç aksiyonları ayrı bir servis üzerinden çağrılacak; UI doğrudan işletim sistemi komutu çalıştırmayacak.

## Teknik seçimler

- Dil: Rust
- Terminal UI: Ratatui
- Terminal backend/input: Crossterm
- Genel sistem metrikleri: sysinfo
- Platforma özel macOS ölçümleri: küçük, izole native API modülleri
- Paketleme: cargo install ile kaynak kurulumu; ilk dağıtımda macOS binary/release arşivi; ileride Homebrew tap
- Lisans: varsayılan olarak MIT; repo public olmadan önce kullanıcı tarafından değiştirilebilir

## Dağıtım ve repo düzeni

Repo başlangıçta private kalacak. İlk düzenlemeler:

- anlaşılır bir README
- MIT LICENSE
- CONTRIBUTING.md ile geliştirme akışı
- CHANGELOG.md
- `.gitignore` ve Rust proje metadata'sı
- GitHub Actions ile format, test ve clippy kontrolü
- GitHub repo açıklaması, topic'ler ve görünür README başlığı

README; kısa ürün açıklaması, ekran görüntüsü/mockup, özellikler, kurulum, klavye kısayolları, macOS gereksinimleri, geliştirme komutları, lisans ve katkı bölümünü içerecek. Gerçek ekran görüntüsü oluşana kadar tasarım mockup'ı açıkça “design preview” olarak etiketlenecek.

## Hata yönetimi

- Tek bir collector hatası diğer panelleri durdurmayacak.
- Desteklenmeyen metrikler `N/A` ve kısa açıklama ile gösterilecek.
- Terminal boyutu yetersizse sadeleştirilmiş görünüm kullanılacak; uygulama çökmeyecek.
- Süreç aksiyonlarında izin hatası kullanıcıya anlaşılır biçimde gösterilecek.
- Çıkışta terminal raw mode ve alternate screen güvenli biçimde restore edilecek.

## Test ve doğrulama

- Collector dönüşümleri ve yüzde/saniye hesapları unit testleriyle doğrulanacak.
- Snapshot üretimi mock veriyle test edilecek.
- Filtreleme, sıralama ve aksiyon onay state'i unit test edilecek.
- UI için en azından farklı terminal boyutlarında render smoke testleri çalıştırılacak.
- `cargo fmt --check`, `cargo clippy -- -D warnings` ve `cargo test` CI'da zorunlu olacak.
- Gerçek macOS üzerinde manuel kontrol: canlı yenileme, terminal resize, süreç aksiyonu, izin hatası ve temiz çıkış.

## Aşamalı teslimat

1. Rust CLI iskeleti, terminal lifecycle ve statik dashboard.
2. Snapshot modeli ve CPU/RAM/disk/ağ collector'ları.
3. Süreç tablosu, filtreleme, sıralama ve detay görünümü.
4. Batarya/sıcaklık ve macOS özel collector'ları.
5. Güvenli süreç aksiyonları ve olay mesajları.
6. README, lisans, CI, release paketi ve GitHub repo düzeni.

Bu sıralama, arayüzü erken doğrularken platforma özel ölçüm ve yıkıcı aksiyonların çekirdeği gereksiz yere karmaşıklaştırmasını önler.
