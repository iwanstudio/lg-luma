# Kontrola rodzicielska w Lumie: „czas bez telewizora”, limity i zakładka Salon

Stan: projekt do akceptacji, 2026-11-18. Dotyczy trzech miejsc: launcher (`iwanstudio/lg-luma`, gałąź `luma-ha`), integracja (`iwanstudio/ha-luma-bridge`) i konfiguracja Home Assistanta (dashboard `kontrola-rodzicielska`, helpery, automatyzacje).

## Po co

Telewizor LG 55NANO867NA w salonie oglądają wszyscy. W „czasie bez telewizora” (okno godzin ustawiane w HA, np. 13:00–19:00, czyli po powrocie dzieci ze szkoły) dzieci (Marysia, Miłosz) mają oglądać tylko za zgodą Iwana albo Martyny. Telewizor ma przy tym jasno, po polsku, powiedzieć dzieciom, że bez zgody nie oglądają. Poza oknem (19:00–13:00) telewizor działa normalnie i o nic nie pyta. Sam PIN na aplikacjach w LG tego nie robi: nie komunikuje powodu i nie spina się z HA.

W HA jest już system kontroli dla Androidów dzieci: dashboard `kontrola-rodzicielska` z widokami MIŁOSZ i MARYSIA (zezwolenie, harmonogram ciszy, limit dzienny, limity per aplikacja z licznikami „dziś”, sekcja „Prośba” z `script.tv_przyznaj_czas` i `script.tv_odrzuc_prosbe`, powiadomienia z akcjami na telefony rodziców). Ta funkcja ma być jego odpowiednikiem dla LG i dołożyć widok **SALON**.

## Cele

1. W czasie bez telewizora telewizor jest zablokowany: pełnoekranowy ekran Lumy z komunikatem po polsku, polem PIN-u i przyciskiem „Poproś o zgodę”.
2. Odblokowanie: (a) PIN wpisany cyframi na pilocie (tryb rodzica), (b) zgoda z telefonu na określoną liczbę minut (tryb dziecka).
3. Limity: dzienny i per aplikacja, **liczone i egzekwowane tylko w czasie bez telewizora** i tylko w trybie dziecka (zgoda z telefonu). Dorosły po PIN-ie ogląda bez limitów do końca okna. Wartość 0 = bez limitu.
4. Liczniki nie zerują się przy wyłączeniu i włączeniu telewizora, ani przy restarcie HA. Zeruje je dopiero nowa doba (my call: północ).
5. Zawracanie na Lumę: gdy w czasie blokady (lub po wyczerpaniu limitu aplikacji) ktoś wejdzie w zablokowaną aplikację, HDMI albo Live TV, HA przez integrację LG webOS wraca na Lumę. Działa bez przerwy, bez osobnej zgody.
6. Zakładka SALON w dashboardzie `kontrola-rodzicielska`, w układzie jak MARYSIA i MIŁOSZ: zezwolenie, godziny (czas bez telewizora), czas dziś, limit dzienny, limity aplikacji (YouTube, Netflix, Prime Video, Disney+, HBO Max + 3 dodatkowe), sekcja „Prośba” (+15 … +120 min, „Odrzuć”).
7. Całość obsługiwana samym pilotem, bez wskaźnika.

## Nie-cele

- Brak osobnych profili Marysi i Miłosza na LG: jeden wspólny stan „Salon”, jeden PIN rodzica (my call: wspólny dla Iwana i Martyny).
- Nie zmieniamy Live TV, wejść HDMI, skrótów numerycznych LG, asystenta głosowego ani Magicznego pilota.
- Nie tworzymy RTSP/MJPEG ani nie ruszamy podglądu domofonu.
- PIN nie jest przechowywany na telewizorze. Blokada nie trafia do `preferences.json` (omija problem rozjazdu dwóch plików).
- Nie zmieniamy działania istniejącej kontroli dla Androidów Marysi i Miłosza.

## Anty-cele (porażka, nawet jeśli technicznie działa)

- Telewizor zablokowany na stałe, gdy HA jest chwilowo niedostępny.
- PIN odczytywalny z telewizora.
- Back, strzałki lub skrót cyfr (przytrzymanie 0–9) omijają blokadę.
- Dorosły oglądający wieczorem (poza oknem) musi coś wpisywać.
- Liczniki zerują się, gdy ktoś wyłączy i włączy telewizor.
- Zmiana skryptów/automatyzacji, która psuje kontrolę Androidów.

## Ograniczenia

- Luma to natywny Rust/SDL. Ruch HTTP poza wątkiem UI (`bridge.rs`), nowy stan przez `Effect` i `Update`.
- Zmiany Lumy: wersja minor w `appinfo.json`, `package.json` i `Cargo.toml` naraz (zgodnie z regułą użytkownika; `tools/sync-version.mjs --check` w CI); plik changeset; Rust 1.93 z `clippy -D warnings` i `rustfmt` w CI (lokalnie brak Rusta).
- Integracja: nowa wersja w `manifest.json`, teksty w `strings.json` i `translations/pl.json`.
- Autoryzacja endpointów tokenem urządzenia, jak `tiles`, `press`, `alert`. Nie używać `/stream`.
- Polskie napisy w `assets/messages.json` i `src/i18n.rs`; pozostałe języki nie mogą się zepsuć (fallback angielski).
- Wgrać trzeba nowy IPK **i** zaktualizować integrację w HACS; konfigurację HA (helpery, widok) zakłada się w HA.
- Zmiany w HA tworzone dopiero po akceptacji tego dokumentu i bez nadpisywania istniejących obiektów Marysi i Miłosza.

## Podejścia

- **A. HA decyduje, Luma pokazuje (wybrane).** Okno, limity, liczniki i zgody w HA; PIN sprawdza HA; HA zawraca telewizor na Lumę. Jedno źródło prawdy, spójne z systemem dzieci.
- B. Luma trzyma okno i PIN lokalnie. Odrzucone: harmonogram w dwóch miejscach, PIN na telewizorze, nie zatrzyma innych aplikacji.
- C. Tylko HA (toast, wyłączanie). Odrzucone: brak pola PIN i czytelnego ekranu.

Wybrał: Iwan, wariant A. Zawartość „fail-open” (brak blokady przy braku łączności z HA) potwierdził Iwan; router jest w zamkniętej szafie.

## Projekt (wariant A)

### Zasady blokowania (my call tam, gdzie zaznaczone)

Stan „czas bez telewizora” = `schedule` w HA jest `on`.

- **Blokada całego telewizora** (ekran blokady): czas bez telewizora jest aktywny, nie ma zgody (`zezwolenie` wyłączone, `przedluzenie_do` w przeszłości) i nie ma trybu rodzica po PIN-ie.
- **Tryb rodzica (PIN):** odblokowanie do końca bieżącego okna, limity nie obowiązują, nic nie jest liczone do limitów (my call).
- **Tryb dziecka (zgoda z telefonu):** zgoda na wybraną liczbę minut (+15, +30, +45, +60, +90, +120 lub własna). W tym czasie oglądanie liczy się do limitów.
- **Limit dzienny:** łączny czas oglądania w trybie dziecka w oknie tej doby. Po wyczerpaniu: blokada całego telewizora z powodem „Limit dzienny wyczerpany”; kolejną zgodą rodzic dodaje minuty ponad limit (jak `przyznaj_czas` u Androidów).
- **Limit aplikacji:** po wyczerpaniu danej aplikacji HA zawraca na Lumę, gdy ta aplikacja jest na wierzchu, i Luma pokazuje komunikat „Netflix: limit na dziś wyczerpany”. Pozostałe aplikacje działają dalej (my call: blokada tylko tej aplikacji, nie całego telewizora).
- Zawracanie przy blokadzie całego telewizora dotyczy każdej aplikacji i wejścia.

### Liczniki

Czas liczony tylko wtedy, gdy telewizor jest włączony, trwa czas bez telewizora i tryb dziecka, per aplikacja (nazwa/ID z atrybutów `media_player.lg_webos_tv`, integracja `webostv`, obszar Salon; uwaga: `media_player.lg_webos_tv_2` to odtwarzacz Music Assistant, nie ten) oraz łącznie. Telewizor poza zasięgiem lub wyłączony (`off`/`unavailable`) nie jest liczony. Liczniki są trwałe: wyłączenie i włączenie telewizora nie zeruje ich, restart HA też nie. Zeruje je północ. Sposób (np. `history_stats` na pomocniczych `binary_sensor`, albo własny licznik z `Store`) wybiera wykonawca, byleby spełniał te wymagania. Spike na telewizorze musi potwierdzić, że integracja webOS podaje bieżącą aplikację w atrybutach (`source` / identyfikator aplikacji).

### Home Assistant: helpery, automatyzacje, dashboard

Wzorem istniejących obiektów dzieci powstają obiekty „salon” (nazwy orientacyjne, wykonawca może je dopasować): `schedule` „czas bez telewizora”, `input_boolean` zezwolenie i prośba o czas, `input_datetime` przedłużenie do, `input_number` limit dzienny i limity aplikacji (z `input_select` na aplikacje dodatkowe), czujniki czasu dziś (łącznie i per aplikacja), `binary_sensor` „zablokowany” z atrybutami powodu i listy zablokowanych aplikacji. Istniejące `script.tv_przyznaj_czas` i `script.tv_odrzuc_prosbe` oraz automatyzacja odpowiedzi z powiadomienia dostają obsługę dziecka „salon” bez zmiany zachowania dla `marysi` i `milosza`.

W dashboardzie `kontrola-rodzicielska` dochodzi widok **SALON** (ścieżka `salon`) w układzie MARYSIA/MIŁOSZ: zezwolenie, telewizor, czas bez telewizora, gauge czasu, limit dzienny, limity aplikacji z licznikami „dziś”, sekcja „Prośba”, opis zasad. Sekcja „Ochrona aplikacji” (PIN stick-a) nie dotyczy LG; zamiast niej przycisk/pole do ustawienia PIN-u rodzica dla Lumy.

### Integracja `luma_bridge`

Nowy krok opcji „Kontrola rodzicielska”: encja blokady (`binary_sensor ... zablokowany`), PIN, encja `media_player` LG, usługi `notify` (telefony Iwana i Martyny), czasy zgód.

Endpointy (token urządzenia):

- `GET /api/luma_bridge/lock` → `{locked, reason, until, pending, denied, blocked_apps, notice}`. `reason` i `notice` po polsku, np. „Czas bez telewizora do 19:00”, „Netflix: limit na dziś wyczerpany”.
- `POST /api/luma_bridge/lock/unlock` z `{pin}` → `{ok, retry_after}`. Porównanie w czasie stałym, PIN jako sól + skrót w HA (my call), po 3 błędnych próbach przerwa 60 s po stronie serwera. Sukces = tryb rodzica do końca okna.
- `POST /api/luma_bridge/lock/request` → powiadomienie push na telefony rodziców z akcjami „+30 min”, „+1 h”, „Odrzuć” (te same nazwy akcji co dla Androidów, z dzieckiem „salon”). Jedna prośba na 120 s. Odrzucenie: telewizor zostaje zablokowany, Luma pokazuje „Rodzice odmówili” (my call).

Zawracanie: dopóki jest blokada całego telewizora, albo bieżąca aplikacja jest na liście `blocked_apps`, a bieżący ekran to nie Luma, HA wciska przycisk Home przez integrację LG webOS (`webostv.button` z `HOME`; ten sam przycisk jest w pilocie widoku Salon, a Luma przejmuje Home, więc kończy się to na Lumie). Reakcja na zmianę stanu/źródła `media_player.lg_webos_tv` z krótkim opóźnieniem (≈2 s), bez pętli odpytywania. Jeśli integracja LG jest niedostępna, HA nic nie robi.

### Luma

- `bridge.rs`: osobne odpytywanie `/lock` co ok. 5 s, niezależne od odświeżania kafelków. Nowe `Update::Lock {...}` i `Update::LockResult {...}`; zadania `LockRequest` i `LockUnlock(pin)`.
- Fail-open: po 3 kolejnych błędach (≈15 s) blokada znika; do tego czasu zachowuje ostatni stan.
- `state.rs`: `Overlay::Lock`. Dopóki aktywny, przechwytuje wszystko: cyfry 0–9 dopisują PIN (max 8), OK zatwierdza, Back kasuje ostatnią cyfrę i nigdy nie zamyka, strzałki przesuwają fokus między polem PIN a „Poproś o zgodę”. Wyłącza przytrzymanie cyfry (`Overlay::Shortcut`) i nie psuje go po zdjęciu blokady. Ma pierwszeństwo przed `Overlay::Camera` (alert domofonu nie przebija blokady, my call).
- `notice` (np. limit aplikacji) pokazywany jako komunikat na ekranie głównym na kilka sekund, po polsku.
- `view.rs`: pełny ekran blokady. Napis „Telewizor jest zablokowany”, powód z HA, pole PIN jako kropki, przycisk „Poproś o zgodę”, linia stanu („Prośba wysłana, czekaj na odpowiedź”, „Zły PIN”, „Spróbuj za 42 s”, „Rodzice odmówili”). Nawigacja przestrzenna, bez wskaźnika.
- Błędy pokazywane po polsku na ekranie, nie połykane. Nic nie zapisujemy do `preferences.json`.

### Wersje

Luma 2.8.0, integracja 0.3.0 (propozycja, minor).

## Do ustalenia przez wykonawcę

- Sprawdzić na telewizorze, że wciśnięcie Home z HA (`webostv.button`) naprawdę otwiera Lumę także z Netflixa, HDMI i Live TV (Iwan potwierdził, że przycisk Home w pilocie widoku Salon działa).
- Sprawdzić, jakie wartości przyjmuje bieżąca aplikacja w `media_player.lg_webos_tv` (atrybuty `source` / `app_id`; Iwan widzi nazwę aplikacji pod kafelkiem telewizora) i jak je zmapować na aplikacje z limitami. Dziś encja jest `unavailable`, więc jej atrybutów nie widziałem.
- Sposób trwałego liczenia (patrz „Liczniki”), format `grant_until` i skrótu PIN-u.
- Układ graficzny ekranu blokady, dokładne teksty, tłumaczenia pozostałych języków.
- Czas odświeżania `/lock` (5 s to propozycja), opóźnienie zawracania (≈2 s), czas życia flag `pending` i `denied`.
- Zakres testów: jednostkowe w Rust (cyfry, fail-open, brak wpływu na Shortcut), w Pythonie (okno, zgody, limit prób), reszta ręcznie na telewizorze.
