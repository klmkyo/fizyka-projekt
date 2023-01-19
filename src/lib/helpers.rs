use std::{fs, io::Write, path::Path, str::FromStr};

#[derive(Clone, Copy, Debug)]
pub struct XY<T> {
    pub x: T,
    pub y: T,
}

impl XY<f64> {
    pub fn length(&self) -> f64 {
        (self.x.powi(2) + self.y.powi(2)).sqrt()
    }
    pub fn normalize(&self) -> Self {
        let length = self.length();
        XY {
            x: self.x / length,
            y: self.y / length,
        }
    }
    pub fn angle(&self) -> f64 {
        self.y.atan2(self.x)
    }
}

pub fn read_input<T: FromStr>(message: &str) -> T
where
    <T as FromStr>::Err: std::fmt::Debug,
{
    print!("{}", message);
    std::io::stdout()
        .flush()
        .expect("Wystąpił błąd podczas wypisywania");
    let mut x = String::new();
    std::io::stdin()
        .read_line(&mut x)
        .expect("Wystąpił błąd podczas odczytu");
    let x: T = x
        .trim()
        .parse()
        .expect("Nie można przekonwertować do liczby");
    x
}

// print a number, colored based on its value (green, yellow, red), also handle NaN
// limit the string to 4 characters
#[inline(always)]
pub fn print_color(number: f64, max_g: f64, max_y: f64) {
    let color = match number {
        x if x < max_g => 32,
        x if x < max_y => 33,
        _ => 31,
    };
    if number.is_infinite() {
        print!("\x1b[0mINF!\x1b[0m ");
    } else {
        print!("\x1b[{}m{:.2}\x1b[0m ", color, number);
    }
}

// Checks if a point (x, y) is inside the bounds of the grid
#[inline(always)]
pub fn in_bounds<T: PartialOrd>(x: T, y: T, min_x: T, max_x: T, min_y: T, max_y: T) -> bool {
    x > min_x && x < max_x && y > min_y && y < max_y
}

pub fn ensure_files_exist() {
    if !Path::new("output").exists() {
        fs::create_dir("output").unwrap();
    }

    if !Path::new("ladunki_ruchome.txt").exists() {
        fs::write("ladunki_ruchome.txt", "")
            .expect("Wystąpił błą∂ podczas tworzenia pliku ladunki_ruchome.txt");
        let contents = "# Podawanie ilości ruchomych ładunków nie jest potrzebne!
#
# Format:
# <x> <y> <q> <m> <vx> <vy> <ax> <ay>
160 120 -0.0008 1 100 -1000 0 0";
        fs::write("ladunki_ruchome.txt", contents)
            .expect("Wystąpił błąd podczas zapisywania do pliku ladunki_ruchome.txt");
    }

    if !Path::new("ladunki_stacjonarne.txt").exists() {
        fs::write("ladunki_stacjonarne.txt", "")
            .expect("Wystąpił błąd podczas tworzenia pliku ladunki_stacjonarne.txt");
        let contents = "# Podawanie ilości stacjonarnych ładunków nie jest potrzebne!
#
# Format:
# <x> <y> <q>
50 130 -5
120 90 5
200 200 3";
        fs::write("ladunki_stacjonarne.txt", contents)
            .expect("Wystąpił błąd podczas zapisywania do pliku ladunki_stacjonarne.txt");
    }
}

pub const K: f64 = 8.99e9;