# Kontrola rodzicielska (czas bez telewizora) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Zablokować LG w „czasie bez telewizora” ekranem Lumy z PIN-em i prośbą o zgodę, z limitami dziennym i per aplikacja liczonymi w HA, plus widok SALON w dashboardzie `kontrola-rodzicielska`.

**Architecture:** Logika (okno, zgody, limity, liczniki, PIN) w czystym module Pythona `lock.py` integracji `luma_bridge`, testowanym bez HA. Cienka warstwa HA (`lockman.py` + widoki HTTP) czyta stan, liczy czas, wciska Home przez `webostv.button` i wystawia `/lock`. Luma tylko odpytuje `/lock` i rysuje `Overlay::Lock`; fail-open po 3 błędach.

**Tech Stack:** Python 3.12 (HA custom component, pytest bez HA), Rust 1.93/edition 2024 (SDL2, ureq), HA MCP (`ha_config_set_helper`, `ha_config_set_dashboard`, `ha_config_set_script`).

**Spec:** `lg-tv/luma/docs/superpowers/specs/2026-11-18-kontrola-rodzicielska-design.md`

**Odstępstwa od spec (decyzje planu, do wiadomości Iwana):**

- Powstają **nowe** skrypty `script.tv_salon_przyznaj_czas` i `script.tv_salon_odrzuc_prosbe`; istniejące skrypty Androidów (używają `remote.*` i `cleantv://`) zostają nietknięte. Akcje powiadomień to `LUMA_GRANT_<min>` / `LUMA_DENY` obsługiwane przez integrację (istniejąca automatyzacja reaguje tylko na prefiks `TV_`).
- Liczniki trzyma integracja (`Store`), a nie `history_stats`; publikuje je jako stany `sensor.luma_salon_*` (`hass.states.async_set`).

## Global Constraints

- Rust 1.93, edition 2024; `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`; `cargo fmt --all -- --check`; `oxfmt --check .` (CI, `.github/workflows/ci.yml`; lokalnie brak Rusta).
- Wersje: Luma `2.8.0` w `appinfo.json`, `package.json`, `Cargo.toml` naraz (+ `node tools/sync-version.mjs`); integracja `0.3.0` w `manifest.json`.
- Nowy stan Lumy tylko przez `Effect` i `Update`; ruch HTTP poza wątkiem UI (`bridge.rs`).
- Brak PIN-u i stanu blokady w `preferences.json`.
- Endpointy z tokenem urządzenia (`_entry_for_request`), `requires_auth = False` jak istniejące; nie używać `/stream`.
- Polskie teksty w `assets/messages.json` (klucze z `en` i `pl`; `Language::t` ma fallback do `en`).
- Okno „czas bez telewizora” = `schedule.tv_salon_czas_bez_telewizora` `on`. Limity działają tylko w oknie i tylko w trybie dziecka; PIN = tryb rodzica do końca okna, bez limitów i bez liczenia. 0 = bez limitu. Liczniki zerowane o północy (czas lokalny HA).
- Telewizor encja: `media_player.lg_webos_tv` (webostv). **Nie** `lg_webos_tv_2` (Music Assistant).
- Nie zmieniać istniejących obiektów Marysi i Miłosza w HA.
- Fail-open: 3 kolejne błędy `/lock` (≈15 s) = brak blokady w Lumie.
- Hasło SSH i klucze nie trafiają do repozytoriów ani logów.

## Kontrakt HTTP (używany przez Taski 2, 4–7)

- `GET /api/luma_bridge/lock` → `{"locked": bool, "reason": str, "until": str("HH:MM" lub ""), "pending": bool, "denied": bool, "blocked_apps": [str], "notice": str}`
- `POST /api/luma_bridge/lock/unlock` body `{"pin": str}` → `{"ok": bool, "retry_after": int}` (sekundy; 0 gdy brak przerwy)
- `POST /api/luma_bridge/lock/request` body `{}` → `{"ok": bool, "wait": int}` (sekundy do kolejnej prośby)
- Brak/zły token: 401 `{"message":"unauthorized"}`.

## Review Focus

1. PIN: pusty, zawiera litery, dłuższy niż 8 cyfr, z zerami wiodącymi („0042”) — nie może się zgadzać z „42” ani przejść jako wyjątek.
2. Okno przekraczające północ albo kończące się w trakcie oglądania, i zgoda wygasająca w trakcie oglądania — blokada wraca, bez oscylacji.
3. Restart HA lub wyłączenie/włączenie telewizora w oknie — liczniki i zgody zostają; północ w trakcie oglądania zeruje liczniki.
4. Nieznana bieżąca aplikacja (`source` to wejście, np. „HDMI 1”, albo `None`) — w blokadzie całego telewizora i tak wraca na Lumę; przy limitach aplikacji nic nie jest liczone ani blokowane.
5. Luma: wielokrotne Back, przytrzymanie Back (Recents) i przytrzymanie cyfry podczas blokady oraz dzwonek domofonu — nic nie omija ani nie zamyka blokady.

---

## Część 1: integracja `luma_bridge` (repo `E:\Claude Sessions\LUMA LG\ha-luma-bridge`)

### Task 1: Czysta logika blokady `lock.py`

**Files:**

- Create: `custom_components/luma_bridge/lock.py`
- Create: `tests/test_lock.py` (pytest, zero importów HA; `pytest.ini` z `pythonpath = custom_components/luma_bridge`)

**Interfaces:**

- Produces:
  - `hash_pin(pin: str, salt: bytes) -> str` (hex SHA-256 z PBKDF2, 100k iteracji), `new_salt() -> bytes`, `verify_pin(pin: str, salt: bytes, digest: str) -> bool` (`hmac.compare_digest`; zwraca False dla pustego/nie-cyfr/>8 znaków).
  - `class PinGuard: __init__(self, max_fails: int = 3, lockout: int = 60)`; `check(now: float) -> int` (pozostałe sekundy przerwy, 0 = wolno); `fail(now: float) -> None`; `ok() -> None`.
  - `@dataclass class LockInputs`: `window_on: bool`, `now: float`, `window_ends: float | None`, `allowed: bool` (zezwolenie), `kid_until: float` (przedłużenie_do, timestamp), `parent_until: float`, `daily_limit_s: int` (0 = brak), `daily_used_s: int`, `app_limits_s: dict[str, int]`, `app_used_s: dict[str, int]`, `current_app: str | None` (klucz limitu lub None), `tv_on: bool`.
  - `@dataclass class LockResult`: `locked: bool`, `reason: str`, `until: str`, `blocked_apps: list[str]`, `kid_mode: bool`.
  - `evaluate(i: LockInputs) -> LockResult`.
  - `class UsageCounter: __init__(self, day: str)`; `tick(self, day: str, seconds: int, app: str | None, counting: bool) -> None` (zeruje przy zmianie `day`); `total: int`, `apps: dict[str, int]`; `to_dict() / from_dict()`.
  - `APP_KEYS: dict[str, str]` (nazwa webOS małymi literami → klucz: `youtube`, `netflix`, `prime_video`, `disney`, `hbo_max`); `app_key(source: str | None, extras: dict[str, str]) -> str | None`.

- [ ] **Step 1: Write failing tests** w `tests/test_lock.py`: `test_pin_leading_zero_not_equal`, `test_pin_rejects_non_digits_and_too_long`, `test_pin_guard_locks_after_three_failures` (3×`fail(100)` → `check(101) == 59`, `check(161) == 0`), `test_outside_window_never_locked`, `test_window_without_grant_locked_with_reason_and_until` (reason zawiera „Czas bez telewizora”, `until == "19:00"` dla `window_ends` o 19:00 lokalnie), `test_parent_unlock_ends_with_window_and_ignores_limits`, `test_kid_grant_counts_and_daily_limit_locks` (limit 5400 s, used 5400 → locked, reason „Limit dzienny wyczerpany”), `test_app_limit_blocks_only_that_app` (`blocked_apps == ["Netflix"]`, `locked` False), `test_zero_limit_means_unlimited`, `test_unknown_app_not_counted_or_blocked`, `test_counter_resets_on_new_day_and_survives_tv_off` (tick z `counting=False` nie dodaje, zmiana dnia zeruje), `test_grant_expiring_exactly_now_is_locked`.
- [ ] **Step 2: Run** `python -m pytest tests/test_lock.py -v` → FAIL (moduł nie istnieje).
- [ ] **Step 3: Implement** `lock.py` według powyższych sygnatur; reguły z sekcji „Zasady blokowania” spec (blokada całego TV tylko w oknie bez zgody; limit dzienny wyczerpany w trybie dziecka = blokada całego TV; limit aplikacji = tylko `blocked_apps`). Teksty po polsku.
- [ ] **Step 4: Run** testy → wszystkie PASS.
- [ ] **Step 5: Commit** `git add custom_components/luma_bridge/lock.py tests pytest.ini && git commit -m "Dodaj czystą logikę blokady (okno, zgody, limity, PIN)"`.

### Task 2: Warstwa HA: `lockman.py`, widoki, opcje

**Files:**

- Create: `custom_components/luma_bridge/lockman.py`
- Modify: `custom_components/luma_bridge/__init__.py` (rejestracja widoków przy `register_view` ok. l. 38–45; wywołanie `LockManager` w `async_setup_entry` i sprzątanie w `async_unload_entry`)
- Modify: `custom_components/luma_bridge/config_flow.py` (menu `init` w l. ~175: dodać `"parental"`; nowy `async_step_parental`)
- Modify: `custom_components/luma_bridge/strings.json`, `translations/pl.json` (krok `parental`: etykiety po polsku)
- Test: ręczny (patrz Step 4); logika już pokryta w Task 1.

**Interfaces:**

- Consumes: `lock.evaluate`, `LockInputs`, `LockResult`, `PinGuard`, `UsageCounter`, `verify_pin`, `hash_pin`, `new_salt`, `app_key`.
- Produces: `class LockManager: __init__(self, hass, entry)`; `async start() -> None`; `async stop() -> None`; `state() -> dict` (kontrakt `GET /lock`); `async unlock(pin: str) -> dict`; `async request() -> dict`; widoki `LumaLockView` (GET `/api/luma_bridge/lock`), `LumaLockUnlockView` (POST `.../lock/unlock`), `LumaLockRequestView` (POST `.../lock/request`).
- Opcje wpisu (`entry.options`): `parental_media_player` (`media_player.lg_webos_tv`), `parental_notify` (lista usług `notify.*`), `parental_pin_hash` + `parental_pin_salt` (zapis PIN-u: pole `parental_pin` w formularzu jest hashowane i nie zapisywane jawnie), `parental_enabled` (bool).

- [ ] **Step 1:** W `lockman.py` zaimplementować: co 10 s (`async_track_time_interval`) `tick` — `UsageCounter.tick(today, 10, app, counting)` gdzie `counting` = TV włączony (`state not in {off, unavailable, unknown}`) ∧ okno ∧ tryb dziecka; zapis do `helpers.storage.Store` (`luma_bridge.lock`, wersja 1): licznik, `parent_until`, `denied_until`, `pending_since`. Po każdym ticku i zmianie stanu `media_player` (`async_track_state_change_event`, opóźnienie ≈2 s) policzyć `evaluate(...)`; gdy `locked` lub bieżąca aplikacja ∈ `blocked_apps`, a `app_id` nie należy do `{"com.ecklf.luma", "com.webos.app.home"}` i TV jest włączony → `hass.services.async_call("webostv", "button", {"entity_id": mp, "button": "HOME"})`. Publikować `sensor.luma_salon_czas_dzis` (minuty, +atrybuty per aplikacja) i `binary_sensor.luma_salon_zablokowany` przez `hass.states.async_set`.
- [ ] **Step 2:** Wejścia czytać z encji `input_boolean.tv_salon_zezwolenie`, `input_datetime.tv_salon_przedluzenie_do`, `input_number.tv_salon_limit_dzienny_h`, `input_number.tv_salon_limit_{youtube,netflix,prime_video,disney,hbo_max}_h`, `input_select.tv_salon_aplikacja_dodatkowa_{1,2,3}` + `input_number.tv_salon_limit_dodatkowej_{1,2,3}_h`, `schedule.tv_salon_czas_bez_telewizora` (`window_ends` z atrybutu `next_event`). Brak encji = limit 0.
- [ ] **Step 3:** `request()` wysyła przez `parental_notify` powiadomienie z akcjami `LUMA_GRANT_30`, `LUMA_GRANT_60`, `LUMA_DENY` (tag `luma_prosba_salon`), jedna prośba na 120 s (`wait`), ustawia `pending`. Nasłuch `mobile_app_notification_action`: `LUMA_GRANT_<m>` → `script.tv_salon_przyznaj_czas` (`minutes: m`), `LUMA_DENY` → `script.tv_salon_odrzuc_prosbe`; ignoruje inne akcje; powtórzone kliknięcie jest idempotentne (drugi `GRANT` dodaje czas tylko, jeśli prośba była `pending`).
- [ ] **Step 4: Verify (ręcznie, po wdrożeniu HACS + restarcie HA):** z komputera `curl -s -H "X-Luma-Token: <token urządzenia>" https://<HA>/api/luma_bridge/lock` zwraca JSON z kontraktu (`locked` zgodne ze stanem `schedule`); zły PIN ×3 zwraca `retry_after > 0`; `POST .../lock/request` powoduje push na telefonie.
- [ ] **Step 5: Commit** `git commit -m "Dodaj blokadę: widoki /lock, liczniki, zawracanie Home, opcje"`.

### Task 3: Wydanie integracji 0.3.0

**Files:** Modify `custom_components/luma_bridge/manifest.json` (`"version": "0.3.0"`), `README.md` (krótki opis kontroli rodzicielskiej).

- [ ] **Step 1:** Podbić wersję, dopisać w README sekcję „Kontrola rodzicielska” (nazwy encji `tv_salon_*`, endpointy).
- [ ] **Step 2:** `python -m pytest tests -q` → PASS; `python -m py_compile custom_components/luma_bridge/*.py` bez błędów.
- [ ] **Step 3: Commit i push** na `main` `iwanstudio/ha-luma-bridge` (`gh` jako `iwanstudio`). Powiedzieć Iwanowi: zaktualizować HACS i zrestartować HA.

## Część 2: konfiguracja HA (przez MCP, bez ruszania obiektów dzieci)

### Task 4: Helpery „salon”

**Files:** brak (zmiany w HA). Weryfikacja: `ha_get_state` dla każdej encji.

- [ ] **Step 1:** Utworzyć (`ha_config_set_helper`): `schedule.tv_salon_czas_bez_telewizora` (pon–pt 13:00–19:00, do edycji), `input_boolean.tv_salon_zezwolenie`, `input_boolean.tv_salon_prosba_o_czas`, `input_datetime.tv_salon_przedluzenie_do` (data+czas), `input_number.tv_salon_limit_dzienny_h` (0–6, krok 0,25, domyślnie 1,5), `input_number.tv_salon_przyznaj_minuty` (0–240, krok 15), `input_number.tv_salon_limit_{youtube,netflix,prime_video,disney,hbo_max}_h` (0–6, krok 0,25, domyślnie 0), `input_select.tv_salon_aplikacja_dodatkowa_{1,2,3}` + `input_number.tv_salon_limit_dodatkowej_{1,2,3}_h`.
- [ ] **Step 2:** `ha_get_state` każdej: istnieje i ma oczekiwany stan początkowy (np. limit dzienny `1.5`).

### Task 5: Skrypty salonu i widok SALON

**Files:** brak (zmiany w HA).

**Interfaces:**

- Produces: `script.tv_salon_przyznaj_czas` (pole `minutes`, puste = `input_number.tv_salon_przyznaj_minuty`): ustawia `input_datetime.tv_salon_przedluzenie_do` na `max(teraz, obecne) + minutes`, wyłącza `input_boolean.tv_salon_prosba_o_czas`, czyści powiadomienia z tagiem `luma_prosba_salon` na telefonach; `script.tv_salon_odrzuc_prosbe`: wyłącza prośbę, ustawia flagę odmowy (zdarzenie `luma_bridge_denied`), czyści powiadomienia.

- [ ] **Step 1:** Utworzyć oba skrypty (`ha_config_set_script`), w stylu istniejącego `tv_przyznaj_czas` (bez `remote.*`).
- [ ] **Step 2:** Dodać widok `SALON` (`path: salon`) do dashboardu `kontrola-rodzicielska` (`ha_config_set_dashboard` z `python_transform`, dopisując widok po MARYSIA; istniejące widoki bez zmian): zezwolenie, `media_player.lg_webos_tv`, schedule, gauge `sensor.luma_salon_czas_dzis`, limit dzienny, limity aplikacji (suwak + licznik „dziś” z atrybutów `sensor.luma_salon_czas_dzis`), sekcja „Prośba” (+15 … +120, `script.tv_salon_przyznaj_czas`; „Odrzuć” → `script.tv_salon_odrzuc_prosbe`), pole do ustawienia PIN-u (formularz opcji integracji), opis zasad.
- [ ] **Step 3: Verify:** `ha_config_get_dashboard(url_path="kontrola-rodzicielska")` — widoki `milosz`, `marysia` identyczne jak przed zmianą (`config_hash` sekcji bez zmian), `salon` obecny. Przycisk +15 min ustawia `przedluzenie_do` na ≈ teraz+15 min.

## Część 3: Luma (repo `E:\Claude Sessions\LUMA LG\lg-tv\luma`, gałąź `luma-ha`)

### Task 6: Stan blokady i zadania w `bridge.rs`

**Files:** Modify `src/bridge.rs` (`Update` l. 61, `Job` l. 79, `Service` l. 86–148, `worker` l. 157); Test `src/tests.rs` lub nowy `src/lock-tests.rs` (`#[path]` jak `recents-tests.rs`).

**Interfaces:**

- Produces: `#[derive(Clone, Debug, Default, PartialEq)] pub struct LockState { pub locked: bool, pub reason: String, pub until: String, pub pending: bool, pub denied: bool, pub blocked_apps: Vec<String>, pub notice: String }`; `Update::Lock(LockState)`; `Update::LockResult { ok: bool, retry_after: u64, error: String }`; `Service::lock_request(&self)`; `Service::lock_unlock(&self, pin: String)`; `fn parse_lock(value: &serde_json::Value) -> Option<LockState>`.

- [ ] **Step 1: Write failing tests:** `parse_lock_reads_contract` (JSON z kontraktu → pola), `parse_lock_missing_fields_defaults_unlocked`, `parse_lock_rejects_non_object` (`None`), `three_failures_make_fail_open` (funkcja `LockPoll { failures: u8 }::record(ok: bool) -> Option<LockState>` zwraca `Some(LockState::default())` dopiero przy 3. błędzie z rzędu i `None` wcześniej).
- [ ] **Step 2:** `cargo test --offline -p luma lock` → FAIL (CI, jeśli brak lokalnie Rusta; sprawdzić przez Actions).
- [ ] **Step 3: Implement:** `GET /lock` co ok. 5 s w pętli `worker` (osobny licznik czasu, niezależny od odpytywania kafelków; błąd HTTP = `record(false)`); `Job::LockRequest` i `Job::LockUnlock(pin)` wywołują POST i wysyłają `Update::LockResult`; błąd sieci → `LockResult { ok: false, retry_after: 0, error: "Brak połączenia z Home Assistant" }`.
- [ ] **Step 4:** Testy PASS (CI). **Step 5: Commit** `git commit -m "Luma: stan blokady i zadania sieciowe w bridge.rs"`.

### Task 7: `Overlay::Lock` w `state.rs` i obsługa klawiszy w `main.rs`

**Files:** Modify `src/state.rs` (`Overlay` l. 111, `Target` l. 34, `Effect` l. 152, `direction` l. 708, `back` l. 2081, `activate` l. 1708, aplikowanie `Update` ok. l. 1542, pola `State`), `src/main.rs` (dispatch efektów l. 372–390, klawisze l. 1114); Test `src/tests.rs`.

**Interfaces:**

- Consumes: `LockState`, `Update::Lock`, `Update::LockResult`.
- Produces: `Overlay::Lock`; `Target::LockPin`, `Target::LockRequest`; `Effect::LockUnlock(String)`, `Effect::LockRequest`; `State` pola `lock: LockState`, `lock_pin: String`, `lock_status: String`; metody `State::locked(&self) -> bool`, `State::lock_digit(&mut self, digit: u8) -> Vec<Effect>`, `State::lock_notice(&self) -> Option<&str>`.

- [ ] **Step 1: Write failing tests** (`src/tests.rs`): `lock_update_opens_overlay_and_unlock_closes_it`; `lock_digits_append_up_to_eight_and_ok_submits` (po `lock_digit` ×4 i `activate()` na `LockPin` → `vec![Effect::LockUnlock("1234")]`); `lock_back_deletes_last_digit_and_never_closes` (3× `back()` przy pustym PIN-ie: `locked()` nadal prawda); `lock_arrows_move_between_pin_and_request`; `lock_blocks_shortcut_and_recents_hold` (podczas blokady przytrzymanie Back nie otwiera Recents; `activate_shortcut_digit` nie uruchamia aplikacji); `lock_overrides_camera_overlay` (`Update::Ring` przy blokadzie nie ustawia `Overlay::Camera`); `lock_result_wrong_pin_sets_polish_status_and_clears_pin`; `lock_result_with_retry_after_shows_seconds`; `unlocked_update_while_locked_restores_no_overlay`.
- [ ] **Step 2:** `cargo test --offline -p luma lock` → FAIL (CI).
- [ ] **Step 3: Implement:** `Update::Lock(s)` → jeśli `s.locked` ustaw `overlay = Some(Overlay::Lock)`, `focus = Target::LockPin`; jeśli nie i overlay to Lock → `None`; `notice` niepusty → `self.notify(notice)` raz na zmianę. `back()` przy `Overlay::Lock` kasuje ostatnią cyfrę i nic poza tym; `direction()` przy Lock przełącza fokus; `Update::Ring` ignorowany przy Lock; w `main.rs` gałąź klawiszy cyfr: `if matches!(state.overlay, Some(Overlay::Lock))` → `pending = state.lock_digit(digit)` przed istniejącymi gałęziami; hold-Back (Recents) ignorowany, gdy `state.locked()`; `Effect::LockUnlock(pin)` → `bridge.lock_unlock(pin)`, `Effect::LockRequest` → `bridge.lock_request()`.
- [ ] **Step 4:** Testy PASS (CI). **Step 5: Commit** `git commit -m "Luma: nakładka blokady, PIN z pilota, prośba o zgodę"`.

### Task 8: Ekran blokady w `view.rs` i teksty

**Files:** Modify `src/view.rs` (listy `Overlay` l. 375–391 oraz nowa `fn lock_screen(out: &mut Out, state: &State, assets: &Assets)`), `assets/messages.json`; Test `src/tests.rs`.

**Interfaces:**

- Consumes: `State.lock`, `State.lock_pin`, `State.lock_status`, `Target::LockPin`, `Target::LockRequest`.
- Produces: klucze i18n (`en` i `pl`): `lockTitle` („Telewizor jest zablokowany”), `lockHint` („Wpisz PIN rodzica pilotem albo poproś o zgodę”), `lockRequest` („Poproś o zgodę”), `lockRequestSent` („Prośba wysłana, czekaj na odpowiedź”), `lockWrongPin` („Zły PIN”), `lockWait` („Spróbuj za {seconds} s”), `lockDenied` („Rodzice odmówili”), `lockOffline` („Brak połączenia z Home Assistant”).

- [ ] **Step 1: Write failing tests:** `lock_screen_scene_has_pin_and_request_controls` (scena z `Overlay::Lock` ma dokładnie kontrolki `Target::LockPin` i `Target::LockRequest`, żadnych innych), `lock_messages_exist_in_polish_and_english` (każdy z 8 kluczy ma `pl` i `en`, `pl` ≠ klucz), `lock_screen_shows_reason_from_ha` (tekst `reason` występuje w scenie), `lock_pin_is_drawn_as_dots_never_digits`.
- [ ] **Step 2:** `cargo test --offline -p luma lock_screen lock_messages` → FAIL (CI).
- [ ] **Step 3: Implement:** pełnoekranowy ciemny panel w logicznym układzie 1920×1080: tytuł, `reason` (i `until`), ośmiopozycyjne pole PIN (kropki), przycisk „Poproś o zgodę”, linia statusu; obramowanie fokusu jak w pozostałych kontrolkach; `Overlay::Lock` dodane do `match`ów (wyczerpujących). Teksty dopisać do `messages.json` bez ruszania innych kluczy.
- [ ] **Step 4:** Testy PASS (CI). **Step 5: Commit** `git commit -m "Luma: ekran blokady po polsku"`.

### Task 9: Wersja 2.8.0, changeset, build i IPK

**Files:** Modify `appinfo.json`, `package.json`, `Cargo.toml`, `Cargo.lock` (wpis `luma`), `readme.md` (przez `node tools/sync-version.mjs`); Create `.changeset/<nazwa>.md`.

- [ ] **Step 1:** Podbić `2.7.10` → `2.8.0` w trzech plikach, `node tools/sync-version.mjs`, `node tools/sync-version.mjs --check` → „Release versions match 2.8.0”. Dodać changeset (minor: „Kontrola rodzicielska: ekran blokady, PIN, prośba o zgodę”).
- [ ] **Step 2:** `npx --yes oxfmt@0.73.0 --check .` → bez uwag (lokalnie); push na `luma-ha`; `gh workflow run CI -R iwanstudio/lg-luma --ref luma-ha`.
- [ ] **Step 3:** Jeśli CI padnie, naprawić na gałęzi (formatowanie, clippy 1.93, testy) i powtórzyć do zielonego.
- [ ] **Step 4:** `gh run download <run> -n luma-release -D "E:\Claude Sessions\LUMA LG\ci-build-2.8.0"`; zweryfikować `sha256sum` z plikiem `sha256sums`; podać Iwanowi dokładny rozmiar w bajtach i polecenia `scp` / `luna-send` (instalacja: `home.sh disable` → `luna-send ... install` → `home.sh enable`).

## Część 4: weryfikacja na żywo (ręcznie, Iwan + ja)

### Task 10: Test na telewizorze

- [ ] **Step 1:** Wgrać IPK 2.8.0 i zaktualizować integrację; w opcjach ustawić `media_player.lg_webos_tv`, telefony i PIN.
- [ ] **Step 2:** Ustawić okno tak, by trwało teraz. Oczekiwane: w ≤10 s Luma pokazuje blokadę; wejście w Netflix/HDMI/Live TV wraca na Lumę w ≈2–5 s; zły PIN ×3 → „Spróbuj za 60 s”; poprawny PIN odblokowuje do końca okna.
- [ ] **Step 3:** „Poproś o zgodę” → push na telefon; „+30 min” zdejmuje blokadę; po upływie wraca.
- [ ] **Step 4:** Przy zgodzie 30 min wyczerpać limit aplikacji (ustawić 0,05 h): aplikacja zamyka się, Luma pokazuje „… limit na dziś wyczerpany”; wyłączenie i włączenie telewizora nie zeruje licznika.
- [ ] **Step 5:** Spisać wartości `source`/`app_id` z `ha_get_state media_player.lg_webos_tv` dla każdej aplikacji i poprawić `APP_KEYS` w `lock.py`, jeśli się różnią (osobny commit + wydanie 0.3.1 tylko jeśli potrzebne).
- [ ] **Step 6:** Odłączyć telewizor od sieci/HA na ≈20 s: blokada znika (fail-open); po powrocie łączności wraca.

---

## Self-Review

- **Pokrycie spec:** okno/PIN/zgoda (T1, T2, T7, T8), limity i liczniki trwałe (T1, T2), zawracanie Home (T2), SALON (T4, T5), fail-open (T6, T10), Back/Recents/Shortcut/Camera (T7), wersje (T3, T9), weryfikacja zakresu „do ustalenia” (T10 Step 5).
- **Spójność typów:** `LockState`/`Update::Lock` (T6) użyte w T7; `Target::LockPin/LockRequest` (T7) w T8; kontrakt HTTP wspólny dla T2, T6.
- **Luki świadome:** lokalnie brak Rusta i HA-testów; kompilację i testy Rusta sprawdza CI, część integracji tylko ręcznie (T2 Step 4, T10).
