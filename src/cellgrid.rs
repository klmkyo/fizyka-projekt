use std::{fs, f64::INFINITY, io::{Write, BufWriter}};

use crate::{lib::helpers::{print_color, XY}, movable_charge::{field_intensity_movable, MovableCharge}};

#[derive(Clone)]
pub struct Cell {
    pub q: f64,
    pub e: XY<f64>,
    // v = potencjał pola elektrycznego
    pub v: f64,
}

struct CellData {
    pub intensity: XY<f64>,
    pub potential: f64,
}

pub struct StationaryCharge {
    pub x: usize,
    pub y: usize,
    pub q: f64,
}

// MovementStep is used to track the movement of movable charges, so that they can be saved to a file (if user wants to)
struct MovementStep {
    pub x: f64,
    pub y: f64,
    pub v: XY<f64>,
    pub a: XY<f64>,
}

pub struct CellGrid {
    w: usize,
    h: usize,
    pub cells: Vec<Vec<Cell>>,
    pub stationary_charges: Vec<StationaryCharge>,
    pub movable_charges: Vec<MovableCharge>,
    // movement stuff
    pub track_movement: bool,
    movement_history: Vec<Vec<MovementStep>>,
}

impl CellGrid {
    fn new(x: usize, y: usize, save_movement: bool) -> Self {
        let cells = vec![
            vec![
                Cell {
                    q: 0.0,
                    e: XY { x: 0.0, y: 0.0 },
                    v: 0.0
                };
                x
            ];
            y
        ];
        CellGrid {
            w: x,
            h: y,
            cells,
            stationary_charges: Vec::new(),
            movable_charges: Vec::new(),
            track_movement: save_movement,
            movement_history: Vec::new(),
        }
    }

    pub fn get_dimensions(&self) -> (usize, usize) {
        (self.w, self.h)
    }

    pub fn new_from_file(file: &str, save_movement: bool) -> Self {
        let mut grid = CellGrid::new(256, 256, save_movement);

        let contents = fs::read_to_string(file).expect("Nie można odczytać pliku");
        let lines = contents.lines();
        // let linecount: usize = lines.next().expect("Nie można odczytać liczby ładunków").parse().expect("Nie można przekonwertować liczby ładunków");
        for (i, line) in lines.enumerate() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() != 3 {
                panic!("Nieprawidłowa ilość wartości w linijce {}", i + 2);
            }
            // read the values, and in case of error, print the line number
            let x: usize = parts[0].parse().unwrap_or_else(|_| {
                panic!("Wystąpił problem przy odczytywaniu X w linii {}", i + 2)
            });
            let y: usize = parts[1].parse().unwrap_or_else(|_| {
                panic!("Wystąpił problem przy odczytywaniu Y w linii {}", i + 2)
            });
            let q = parts[2].parse().unwrap_or_else(|_| {
                panic!("Wystąpił problem przy odczytywaniu Q w linii {}", i + 2)
            });
            grid.cells[y][x].q = q;
            grid.stationary_charges.push(StationaryCharge { x, y, q });
        }
        // if grid.stationary_charges.len() != linecount {
        //     panic!("Liczba ładunków nie zgadza się z liczbą w pierwszej linii!");
        // }
        grid
    }

    pub fn populate_field(&mut self) {
        for (y, row) in self.cells.iter_mut().enumerate() {
            for (x, cell) in row.iter_mut().enumerate() {
                let cell_data = field_intensity_potential(&self.stationary_charges, x, y);
                cell.e.x = cell_data.intensity.x;
                cell.e.y = cell_data.intensity.y;
                cell.v = cell_data.potential;
            }
        }
    }

    pub fn save_grid_to_file(&self, file: &str) {
        let mut output_file = fs::File::create(file).expect("Nie można utworzyć pliku");
        for (y, row) in self.cells.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                // format: x, y, charge, Ex, Ey, E, V
                writeln!(
                    output_file,
                    "{:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}",
                    x,
                    y,
                    cell.q,
                    cell.e.x,
                    cell.e.y,
                    cell.e.length(),
                    cell.v
                )
                .expect("Nie można zapisać do pliku");
            }
        }
    }

    fn display_intensity_color(&self) {
        for row in &self.cells {
            for cell in row {
                print_color(cell.e.length(), 0.1, 0.5);
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

    pub fn save_movement_history(&self) {
        // TODO make this work better with multiple charges

        // check if the movement history is enabled
        if !self.track_movement {
            panic!("Nie można zapisać historii ruchu, gdy opcja jest wyłączona!");
        }

        for i in 0..self.movable_charges.len() {
            // output/charge_[i].csv
            let mut output_file = fs::File::create(format!("output/charge_{}.csv", i))
                .expect("Nie można utworzyć pliku");
            {
                let mut output_file_buffer = BufWriter::new(&mut output_file);
                for step in &self.movement_history[i] {
                    writeln!(
                        output_file_buffer,
                        // write with 6 decimal places
                        "{:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}",
                        step.x, step.y, step.v.x, step.v.y, step.a.x, step.a.y
                    )
                    .expect("Nie można zapisać do pliku");
                }
            }
            // remove the last newline
            output_file.flush().expect("Nie można wyczyścić bufora");
        }
    }

    pub fn add_movable_charge(&mut self, charge: MovableCharge) {
        self.movable_charges.push(charge);
        // add a new vector to the movement history
        self.movement_history.push(Vec::new());
    }

    pub fn update_movable_charges(&mut self, delta_t: f64) {
        for (i, movable_charge) in &mut self
            .movable_charges
            .iter_mut()
            .filter(|c| c.should_move)
            .enumerate()
        {
            // TODO popraw żeby raz używało prawidłowo wczesniejszych wartości raz aktualnych
            let intensity = field_intensity_movable(
                movable_charge.x,
                movable_charge.y,
                &self.stationary_charges,
            );
            // if the charge is too close to a stationary charge, field_intensity_movable returns Inf for all values
            // thus why we check only one of them
            // in that case, we don't want to update the charge's position
            if intensity.is_none() {
                movable_charge.collided = true;
                movable_charge.should_move = false;
                continue;
            }
            let intensity = intensity.unwrap();

            movable_charge.x +=
                (movable_charge.v.x * delta_t) + (0.5 * movable_charge.a.x * delta_t.powi(2));
            movable_charge.y +=
                (movable_charge.v.y * delta_t) + (0.5 * movable_charge.a.y * delta_t.powi(2));

            movable_charge.v.x += movable_charge.a.x * delta_t;
            movable_charge.v.y += movable_charge.a.y * delta_t;

            movable_charge.a.x = intensity.x * movable_charge.q / movable_charge.m;
            movable_charge.a.y = intensity.y * movable_charge.q / movable_charge.m;

            if self.track_movement {
                self.movement_history[i].push(MovementStep {
                    x: movable_charge.x,
                    y: movable_charge.y,
                    a: movable_charge.a.clone(),
                    v: movable_charge.v.clone(),
                });
            }
        }
    }
}

// function used to calculate the field intensity and potential generated by
    // stationary charges at a given point (x, y)
    // used for displaying the background, and for calculating the field intensity
    // which is then saved to a file
    #[inline(always)]
    fn field_intensity_potential(
        stationary_charges: &[StationaryCharge],
        x: usize,
        y: usize
    ) -> CellData {
        let mut intensity = XY { x: 0.0, y: 0.0 };
        let mut potential = 0.0;
        for stationary_charge in stationary_charges {
            let r_sq = (((x as i32 - stationary_charge.x as i32).pow(2)
                + (y as i32 - stationary_charge.y as i32).pow(2)) as f64);
            let r = r_sq.sqrt();

            if r == 0.0 {
                return CellData {
                    intensity: XY {
                        x: INFINITY,
                        y: INFINITY,
                    },
                    potential: INFINITY,
                };
            }

            intensity.x +=
                (stationary_charge.q * (x as i32 - stationary_charge.x as i32) as f64) / r_sq;
            intensity.y +=
                (stationary_charge.q * (y as i32 - stationary_charge.y as i32) as f64) / r_sq;
            potential += stationary_charge.q / r;
        }
        CellData {
            intensity,
            potential,
        }
    }