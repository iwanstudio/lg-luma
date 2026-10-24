# Zmiany własnej wersji Lumy

Fork [ecklf/lg-luma](https://github.com/ecklf/lg-luma) na telewizor LG 55NANO867NA (webOS 5, oprogramowanie 04.64.00).

Każda nowa funkcja dostaje tu krótki wpis: co robi, od której wersji i jak się jej używa.

## 2.7.2

- Skróty cyfr 0–9.
- W Aplikacjach przytrzymaj OK na aplikacji. Obok ulubionych i ukrywania jest Przypisz klawisz.
- Wybierz cyfrę. Jedna cyfra należy do jednej aplikacji. Ponowne przypisanie tej samej cyfry zabiera ją poprzedniej.
- Przytrzymaj tę cyfrę przez chwilę, gdy widać Lumę. Aplikacja się otworzy. Krótkie naciśnięcie nic nie robi.
- Usuń skrót jest w tym samym oknie wyboru cyfry.
- Cyfry działają tylko, gdy Luma jest na ekranie. W telewizji nadal zmieniają kanał.

## 2.7.1

Zainstalowana wcześniej na telewizorze.

- Język polski na liście: Ustawienia, Language, Polski.
- Font z polskimi znakami: ą, ć, ę, ł, ń, ó, ś, ź, ż. Data wygląda na przykład tak: ŚRODA, 21 PAŹDZIERNIKA.
- Przycisk Home i start po pełnym wyłączeniu zostają z oryginalnej Lumy (`home.sh`). Osobnego autostartu nie włączać równocześnie.
- Ustawienia zapisują się poza paczką. Są dwa pliki, bo zwykła ikona i nakładka Home piszą jako różni użytkownicy:
  - `/media/developer/com.ecklf.luma/preferences.json`
  - `/media/internal/.com.ecklf.luma/preferences.json`
