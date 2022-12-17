use std::{fs, str::FromStr, io::Write};

fn read_input<T: FromStr>(message: &str) -> T where <T as FromStr>::Err: std::fmt::Debug {
    print!("{}", message);
    std::io::stdout().flush().expect("Wystąpił błąd podczas wypisywania");
    let mut x = String::new();
    std::io::stdin().read_line(&mut x).expect("Wystąpił błąd podczas odczytu");
    let x: T = x.trim().parse().expect("Nie można przekonwertować do liczby");
    x
}

struct XY {
    x: f64,
    y: f64,
}

struct Charge {
    x: f64,
    y: f64,
    q: f64,
    m: f64,
    v: XY,
    a: XY,
}

struct StationaryCharge {
    x: f64,
    y: f64,
    q: f64,
}

impl std::fmt::Display for StationaryCharge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "x: {}, y: {}, q: {}", self.x, self.y, self.q)
    }
}

struct Cell {
    ex: f64,
    ey: f64,
    e: f64,
    v: f64,
}

fn parse_charges(file: &str) -> Vec<StationaryCharge> {
    let contents = fs::read_to_string(file).expect("Nie można odczytać pliku");
    let mut charges = Vec::new();
    let mut lines = contents.lines();
    let linecount: i32 = lines.next().expect("Nie można odczytać liczby ładunków").parse().expect("Nie można przekonwertować liczby ładunków");
    for (i, line) in lines.enumerate() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 3 {
            panic!("Nieprawidłowa ilość wartości w linijce {}", i + 2);
        }
        // read the values, and in case of error, print the line number
        let x = parts[0].parse().expect(&format!("Wystąpił problem przy odczytywaniu X w linii {}", i + 2));
        let y = parts[1].parse().expect(&format!("Wystąpił problem przy odczytywaniu Y w linii {}", i + 2));
        let q = parts[2].parse().expect(&format!("Wystąpił problem przy odczytywaniu Q w linii {}", i + 2));
        charges.push(StationaryCharge { x, y, q });
    }
    if charges.len() as i32 != linecount {
        panic!("Liczba ładunków nie zgadza się z liczbą w pierwszej linii!");
    }
    charges
}

fn main() {
    let charges = parse_charges("ładunki.txt");
    println!("Odczytane ładunki:");
    for charge in charges {
        println!("{}", charge);
    }

    let resolution = read_input::<i32>("Podaj rozdzielczość siatki: ");

    todo!();

    let mut charge: Charge;
    println!("Podaj dane ładunku: ");
    charge.x = read_input(" - położenie początkowe X: ");
    charge.y = read_input(" - położenie początkowe Y: ");
    charge.q = read_input(" - ładunek [C]: ");
    charge.m = read_input(" - masa [kg]: ");
    charge.v.x = read_input(" - prędkość początkowa X: ");
    charge.v.y = read_input(" - prędkość początkowa Y: ");

    // E_x(x_0, y_0) = natężenie elektryczne w punkcie (x_0, y_0)
    charge.a.x = (charge.q * ) / charge.m;

    println!();
    let delta_t = read_input::<f64>("Krok czasowy [s]: ");
}
