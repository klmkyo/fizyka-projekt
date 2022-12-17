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
#[derive(Clone)]
struct Cell {
    ex: f64,
    ey: f64,
    e: f64,
    // v = potencjał pola elektrycznego
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

struct CellData {
    intensity: XY,
    potential: f64,
}

fn field_intensity_potential(x: f64, y: f64, charges: &Vec<StationaryCharge>) -> CellData {
    let mut intensity = XY { x: 0.0, y: 0.0 };
    let mut potential = 0.0;
    for charge in charges {
        let r = ((x - charge.x).powi(2) + (y - charge.y).powi(2)).sqrt();
        intensity.x += (charge.q * (x - charge.x)) / (r.powi(3));
        intensity.y += (charge.q * (y - charge.y)) / (r.powi(3));
        potential += charge.q / r;
    }
    CellData { intensity, potential }
}

// print a number, colored based on its value (green, yellow, red)
fn print_color(number: f64) {
    let color = match number {
        x if x < 0.1 => 32,
        x if x < 0.5 => 33,
        _ => 31,
    };
    print!("\x1b[{}m{:.2}\x1b[0m ", color, number);
}

fn main() {
    let charges = parse_charges("ładunki.txt");
    println!("Odczytane ładunki:");
    for charge in &charges {
        println!("{}", charge);
    }

    // create 2d vector of cells (256x256)
    let mut cells = vec![vec![Cell { ex: 0.0, ey: 0.0, e: 0.0, v: 0.0 }; 32]; 32];

    // calculate field intensity for each cell
    for (x, row) in cells.iter_mut().enumerate() {
        for (y, cell) in row.iter_mut().enumerate() {
            let cell_data = field_intensity_potential(x as f64, y as f64, &charges);
            cell.ex = cell_data.intensity.x;
            cell.ey = cell_data.intensity.y;
            cell.e = (cell_data.intensity.x.powi(2) + cell_data.intensity.y.powi(2)).sqrt();
            // check for NaN
            // if cell.e.is_nan() {
            // }
            cell.v = cell_data.potential;
        }
    }

    // print the field intensity for each cell
    for row in &cells {
        for cell in row {
            print_color(cell.e);
        }
        println!()
    }

    // todo!();

    // let mut charge: Charge;
    // println!("Podaj dane ładunku: ");
    // charge.x = read_input(" - położenie początkowe X: ");
    // charge.y = read_input(" - położenie początkowe Y: ");
    // charge.q = read_input(" - ładunek [C]: ");
    // charge.m = read_input(" - masa [kg]: ");
    // charge.v.x = read_input(" - prędkość początkowa X: ");
    // charge.v.y = read_input(" - prędkość początkowa Y: ");

    // // E_x(x_0, y_0) = natężenie elektryczne w punkcie (x_0, y_0)
    // // charge.a.x = (charge.q * ) / charge.m;

    // println!();
    // let delta_t = read_input::<f64>("Krok czasowy [s]: ");
}
