use std::{fs, str::FromStr, io::{Write, Seek, SeekFrom}, f64::INFINITY, time::Instant, cell, ops::Range};
use macroquad::{prelude::*, ui::{widgets::{self, Group}, root_ui, self}, hash};

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

impl XY<f64> {
    fn length(&self) -> f64 {
        (self.x.powi(2) + self.y.powi(2)).sqrt()
    }
    fn normalize(&self) -> Self {
        let length = self.length();
        XY { x: self.x / length, y: self.y / length }
    }
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

fn field_intensity_potential(x: usize, y: usize, stationary_charges: &Vec<StationaryCharge>) -> CellData {
    let mut intensity = XY { x: 0.0, y: 0.0 };
    let mut potential = 0.0;
    for stationary_charge in stationary_charges {
        let r = (((x as i32 - stationary_charge.x as i32).pow(2) + (y as i32 - stationary_charge.y as i32).pow(2)) as f64).sqrt();

        if r == 0.0 {
            return CellData { intensity: XY { x: INFINITY, y: INFINITY }, potential: INFINITY };
        }

        intensity.x += (stationary_charge.q * (x as i32 - stationary_charge.x as i32) as f64) / (r.powi(3));
        intensity.y += (stationary_charge.q * (y as i32 - stationary_charge.y as i32) as f64) / (r.powi(3));
        potential += stationary_charge.q / r;
    }
    CellData { intensity, potential }
}

fn field_intensity_movable(x: f64, y: f64, stationary_charges: &Vec<StationaryCharge>) -> XY<f64> {
    let mut intensity = XY { x: 0.0, y: 0.0 };
    for stationary_charge in stationary_charges {
        let r = (((x - stationary_charge.x as f64).powi(2) + (y - stationary_charge.y as f64).powi(2)) as f64).sqrt();
        if r == 0.0 {
            return XY { x: INFINITY, y: INFINITY };
        }
        intensity.x += (stationary_charge.q * (x - stationary_charge.x as f64)) / (r.powi(3));
        intensity.y += (stationary_charge.q * (y - stationary_charge.y as f64)) / (r.powi(3));
    }
    intensity
}

// print a number, colored based on its value (green, yellow, red), also handle NaN
// limit the string to 4 characters
#[inline(always)]
fn print_color(number: f64, max_g: f64, max_y: f64) {
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
        let mut grid = CellGrid::new(256, 256);

        let contents = fs::read_to_string(file).expect("Nie można odczytać pliku");
        let mut lines = contents.lines();
        // let linecount: usize = lines.next().expect("Nie można odczytać liczby ładunków").parse().expect("Nie można przekonwertować liczby ładunków");
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
        // if grid.stationary_charges.len() != linecount {
        //     panic!("Liczba ładunków nie zgadza się z liczbą w pierwszej linii!");
        // }
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
    fn display_intensity_color(&self) {
        for row in &self.cells {
            for cell in row {
                print_color(cell.e, 0.1, 0.5);
            }
            println!();
        }
    }
    fn display_potential_color(&self) {
        for row in &self.cells {
            for cell in row {
                print_color(cell.v, 0.1, 0.5);
            }
            println!();
        }
    }
    fn add_movable_charge(&mut self, charge: MovableCharge) {
        self.movable_charges.push(charge);
    }
    fn update_movable_charges(&mut self, delta_t: f64) {
        for movable_charge in &mut self.movable_charges {
            let intensity = field_intensity_movable(movable_charge.x, movable_charge.y, &self.stationary_charges);
            
            movable_charge.a.x = intensity.x * movable_charge.q / movable_charge.m;
            movable_charge.a.y = intensity.y * movable_charge.q / movable_charge.m;

            movable_charge.v.x += movable_charge.a.x * delta_t;
            movable_charge.v.y += movable_charge.a.y * delta_t;

            movable_charge.x += movable_charge.v.x * delta_t;
            movable_charge.y += movable_charge.v.y * delta_t;
        }
    }
}

async fn macroquad_display(cellgrid: &mut CellGrid) {
    let mut steps_by_frame = 1500;
    let mut delta_t = 0.0001;
    let mut paused = false;

    loop {
        let start = Instant::now();
        if !paused {
            for _ in 0..steps_by_frame {
                cellgrid.update_movable_charges(delta_t);
            }
        }
        let update_time = start.elapsed().as_micros();

        // fit the grid to the screen
        let scale_x = screen_width() / (cellgrid.cells.len() as f32);
        let scale_y = screen_height() / (cellgrid.cells[0].len() as f32);
        
        clear_background(BLACK);
        // display intensity
        for (y, row) in cellgrid.cells.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                let intensity = cell.e as f32;
                draw_rectangle(x as f32 * scale_x, y as f32 * scale_y, scale_x, scale_y, Color { r: intensity, g: intensity, b: intensity, a: 255.0 });
            }
        }

        // display stationary charges
        for charge in &cellgrid.stationary_charges {
            draw_circle(charge.x as f32 * scale_x + scale_x/2.0, charge.y as f32 * scale_y + scale_y/2.0, 5.0, RED);
        }
        // display movable charges and draw force vectors as arrows
        for charge in &cellgrid.movable_charges {
            let charge_x_scaled = charge.x as f32 * scale_x + scale_x/2.0;
            let charge_y_scaled = charge.y as f32 * scale_y + scale_y/2.0;
            draw_circle(charge_x_scaled, charge_y_scaled, 5.0, GREEN);

            // draw force vector
            let fx = charge.m * charge.a.x * 100.0;
            let fy = charge.m * charge.a.y * 100.0;
            draw_line(charge_x_scaled, charge_y_scaled, charge_x_scaled + fx as f32 * scale_x, charge_y_scaled + fy as f32 * scale_y, 1.0, YELLOW);

            // draw velocity vector
            let vx = charge.v.x * 4.0;
            let vy = charge.v.y * 4.0;
            draw_line(charge_x_scaled, charge_y_scaled, charge_x_scaled + vx as f32 * scale_x, charge_y_scaled + vy as f32 * scale_y, 1.0, BLUE);

            // show charge values above the charge (rounded to 2 decimal places)
            draw_text(&format!("x: {:.2}, y: {:.2}, q: {:.2}, m: {:.2}, v: ({:.2}, {:.2}), a: ({:.2}, {:.2})", charge.x, charge.y, charge.q, charge.m, charge.v.x, charge.v.y, charge.a.x, charge.a.y), charge_x_scaled, charge_y_scaled - 20.0, 10.0, WHITE);
        }

        // show fps
        draw_text(&format!("FPS: {}", get_fps()), 10.0, 10.0, 20.0, WHITE);
        draw_text(&format!("Obliczenia na klatke: {}ms | Render {}ms", update_time as f64 / 1000.0, get_frame_time()*1000.0), 10.0, 30.0, 20.0, WHITE);

        // pause when Space is pressed
        if is_key_pressed(KeyCode::Space) {
            paused = !paused;
        }

        // TODO:
        // - add a way to change the number of steps per frame
        // - add a way to change delta_t
        // - make a way to add charges in gui


        next_frame().await
    }
}

#[macroquad::main("BasicShapes")]
async fn main() {
    let mut cellgrid = CellGrid::new_from_file("ładunki.txt");
    println!("Odczytane ładunki:");
    for charge in &cellgrid.stationary_charges {
        println!("x: {}, y: {}, q: {}", charge.x, charge.y, charge.q);
    }

    let start = Instant::now();
    cellgrid.populate_field();
    let populate_time = start.elapsed().as_micros();
    cellgrid.save_to_file("output.txt");
    // cellgrid.display_potential_color();
    println!("Czas obliczeń: {}ms", populate_time as f64 / 1000.0);

    cellgrid.add_movable_charge(MovableCharge { x: 0.0, y: 0.0, q: 1.0, m: 1.0, v: XY { x: 0.6, y: 3.0 }, a: XY { x: 0.0, y: 0.0 } });

    macroquad_display(&mut cellgrid).await;
}
