use std::{fs, str::FromStr, io::{Write, Seek, SeekFrom}, f64::INFINITY, time::Instant, cell};

fn read_input<T: FromStr>(message: &str) -> T where <T as FromStr>::Err: std::fmt::Debug {
    print!("{}", message);
    std::io::stdout().flush().expect("Wystąpił błąd podczas wypisywania");
    let mut x = String::new();
    std::io::stdin().read_line(&mut x).expect("Wystąpił błąd podczas odczytu");
    let x: T = x.trim().parse().expect("Nie można przekonwertować do liczby");
    x
}

struct XY<T> {
    x: T,
    y: T,
}

struct MovableCharge {
    x: f64,
    y: f64,
    q: f64,
    m: f64,
    v: XY<f64>,
    a: XY<f64>,
}

#[derive(Clone)]
struct Cell {
    q: f64,
    ex: f64,
    ey: f64,
    e: f64,
    // v = potencjał pola elektrycznego
    v: f64,
}

struct CellData {
    intensity: XY<f64>,
    potential: f64,
}

struct StationaryCharge {
    x: usize,
    y: usize,
    q: f64,
}

fn field_intensity_potential(x: usize, y: usize, charges: &Vec<StationaryCharge>) -> CellData {
    let mut intensity = XY { x: 0.0, y: 0.0 };
    let mut potential = 0.0;
    for charge in charges {
        let r = (((x as i32 - charge.x as i32).pow(2) + (y as i32 - charge.y as i32).pow(2)) as f64).sqrt();

        if r == 0.0 {
            return CellData { intensity: XY { x: INFINITY, y: INFINITY }, potential: INFINITY };
        }

        intensity.x += (charge.q * (x as i32 - charge.x as i32) as f64) / (r.powi(3));
        intensity.y += (charge.q * (y as i32 - charge.y as i32) as f64) / (r.powi(3));
        potential += charge.q / r;
    }
    CellData { intensity, potential }
}

// print a number, colored based on its value (green, yellow, red), also handle NaN
// limit the string to 4 characters
fn print_color(number: f64) {
    let color = match number {
        x if x < 0.1 => 32,
        x if x < 0.5 => 33,
        _ => 31,
    };
    if number.is_infinite() {
        print!("\x1b[0mINF!\x1b[0m ");
    } else {
        print!("\x1b[{}m{:.2}\x1b[0m ", color, number);
    }
}

// create a struct called CellGrid, which is a 2d vector of Cells
struct CellGrid {
    cells: Vec<Vec<Cell>>,
    stationary_charges: Vec<StationaryCharge>,
    movable_charges: Vec<MovableCharge>,
}

impl CellGrid {
    fn new(x: usize, y: usize) -> Self {
        let cells = vec![vec![Cell { q: 0.0, ex: 0.0, ey: 0.0, e: 0.0, v: 0.0 }; x]; y];
        CellGrid { cells, stationary_charges: Vec::new(), movable_charges: Vec::new() }
    }
    fn new_from_file(file: &str) -> Self {
        let mut grid = CellGrid::new(32, 32);

        let contents = fs::read_to_string(file).expect("Nie można odczytać pliku");
        let mut lines = contents.lines();
        let linecount: usize = lines.next().expect("Nie można odczytać liczby ładunków").parse().expect("Nie można przekonwertować liczby ładunków");
        for (i, line) in lines.enumerate() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() != 3 {
                panic!("Nieprawidłowa ilość wartości w linijce {}", i + 2);
            }
            // read the values, and in case of error, print the line number
            let x: usize = parts[0].parse().unwrap_or_else(|_| panic!("Wystąpił problem przy odczytywaniu X w linii {}", i + 2));
            let y: usize = parts[1].parse().unwrap_or_else(|_| panic!("Wystąpił problem przy odczytywaniu Y w linii {}", i + 2));
            let q = parts[2].parse().unwrap_or_else(|_| panic!("Wystąpił problem przy odczytywaniu Q w linii {}", i + 2));
            grid.cells[x][y].q = q;
            grid.stationary_charges.push(StationaryCharge { x: x, y: y, q });
        }
        if grid.stationary_charges.len() != linecount {
            panic!("Liczba ładunków nie zgadza się z liczbą w pierwszej linii!");
        }
        grid
    }
    fn populate_field(&mut self) {
        for (y, row) in self.cells.iter_mut().enumerate() {
            for (x, cell) in row.iter_mut().enumerate() {
                let cell_data = field_intensity_potential(x, y, &self.stationary_charges);
                cell.ex = cell_data.intensity.x;
                cell.ey = cell_data.intensity.y;
                cell.e = (cell_data.intensity.x.powi(2) + cell_data.intensity.y.powi(2)).sqrt();
                cell.v = cell_data.potential;
            }
        }
    }
    fn save_to_file(&self, file: &str) {
        let mut output_file = fs::File::create(file).expect("Nie można utworzyć pliku");
        for (y, row) in self.cells.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                // format: x, y, charge, Ex, Ey, E, V
                writeln!(output_file, "{}, {}, {}, {}, {}, {}, {}", x, y, cell.q, cell.ex, cell.ey, cell.e, cell.v).expect("Nie można zapisać do pliku");
            }
        }
        // remove the last newline
        output_file.seek(SeekFrom::End(-1)).expect("Nie można przesunąć kursora");
        output_file.set_len((&output_file).stream_position().expect("Nie można odczytać pozycji kursora")).expect("Nie można zmienić długości pliku");
        // close the file
        output_file.flush().expect("Nie można wyczyścić bufora");
    }
    fn display_color(&self) {
        for row in &self.cells {
            for cell in row {
                print_color(cell.e);
            }
            println!();
        }
    }
}


fn main() {
    let mut cellgrid = CellGrid::new_from_file("ładunki.txt");
    println!("Odczytane ładunki:");
    for charge in &cellgrid.stationary_charges {
        println!("x: {}, y: {}, q: {}", charge.x, charge.y, charge.q);
    }

    let start = Instant::now();
    cellgrid.populate_field();
    let populate_time = start.elapsed().as_micros();
    cellgrid.save_to_file("output.txt");
    cellgrid.display_color();
    println!("Czas obliczeń: {}ms", populate_time as f64 / 1000.0);

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
