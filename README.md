## Instalacja
```
git clone https://github.com/klmkyo/fizyka-projekt.git
cd fizyka-projekt
cargo run --release --
#                      ^ tutaj można podać parametry
```

## Korzystanie
Jeśli chcemy uruchomić symulację z GUI, wystarczy uruchomić program bez parametrów.

Aby zapisać wyniki do pliku, należy użyć parametru `--zapisz-pole` lub `--zapisz-ruch` (wraz z parametrem `--bez-gui`). Wynik pola zostanie zapisany do `output/output_grid.csv`, a ruch ładunków do `output/charge.csv`.

Można edytować ładunki w odpowiednio w plikach `ladunki_stacjonarne.txt` i `ladunki_ruchome.txt`.

## Parametry do programu
```
      --bez-gui                  Nie pokazuj okna z symulacją
  -m, --max-krokow <MAX_KROKOW>  Maksymalna liczba kroków symulacji [default: 10000]
  -d, --delta-t <DELTA_T>        Przyjęta delta dla symulacji [default: 0.000001]
      --zakoncz-po-opuszczeniu   Czy symulacja powinna być przerwana gdy wszystkie ładunki opuszczą siatkę
      --zapisz-pole              Czy zapisać natężenie pola do pliku
      --zapisz-ruch              Czy zapisać ruch ładunków do pliku (działa tylko bez GUI)
```
