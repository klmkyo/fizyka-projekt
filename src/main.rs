use clap::Parser;
use egui::Pos2;
use macroquad::{self, prelude::*};
use std::{
    f64::INFINITY,
    fs,
    io::{BufWriter, Seek, SeekFrom, Write},
    path::Path,
    str::FromStr,
    time::Instant,
};
extern crate rand;
use rand::{thread_rng, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

mod toggle;

fn read_input<T: FromStr>(message: &str) -> T
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

// remove last character from a file, used to remove the newline character
fn remove_last_char(file: &mut fs::File) {
    file.seek(SeekFrom::End(-1))
        .expect("Wystąpił błąd podczas przesuwania się w pliku");
    file.set_len(
        file.metadata()
            .expect("Wystąpił błąd podczas pobierania metadanych pliku")
            .len()
            - 1,
    )
    .expect("Wystąpił błąd podczas zmniejszania rozmiaru pliku");
}

#[derive(Clone)]
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
        XY {
            x: self.x / length,
            y: self.y / length,
        }
    }
    fn angle(&self) -> f64 {
        self.y.atan2(self.x)
    }
}

struct MovableCharge {
    should_move: bool,
    collided: bool,
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
    e: XY<f64>,
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

// inline
#[inline(always)]
fn field_intensity_potential(
    x: usize,
    y: usize,
    stationary_charges: &Vec<StationaryCharge>,
) -> CellData {
    let mut intensity = XY { x: 0.0, y: 0.0 };
    let mut potential = 0.0;
    for stationary_charge in stationary_charges {
        let r = (((x as i32 - stationary_charge.x as i32).pow(2)
            + (y as i32 - stationary_charge.y as i32).pow(2)) as f64)
            .sqrt();

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
            (stationary_charge.q * (x as i32 - stationary_charge.x as i32) as f64) / (r.powi(3));
        intensity.y +=
            (stationary_charge.q * (y as i32 - stationary_charge.y as i32) as f64) / (r.powi(3));
        potential += stationary_charge.q / r;
    }
    CellData {
        intensity,
        potential,
    }
}

const K: f64 = 8.99e9;

// static mut lowest: f64 = INFINITY;

fn field_intensity_movable(x: f64, y: f64, stationary_charges: &Vec<StationaryCharge>) -> XY<f64> {
    let mut intensity_xy = XY { x: 0.0, y: 0.0 };
    for stationary_charge in stationary_charges {
        let r = ((x - stationary_charge.x as f64).powi(2)
            + (y - stationary_charge.y as f64).powi(2))
        .sqrt();
        if r < 2. {
            return XY {
                x: INFINITY,
                y: INFINITY,
            };
        }
        // if r < unsafe { lowest } {
        //     unsafe {
        //         lowest = r;
        //     }
        //     println!("lowest: {}", unsafe { lowest });
        // }
        let intensity = stationary_charge.q / r.powi(2);
        intensity_xy.x += intensity * (x - stationary_charge.x as f64) / r;
        intensity_xy.y += intensity * (y - stationary_charge.y as f64) / r;
    }
    intensity_xy
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

struct MovementStep {
    x: f64,
    y: f64,
    v: XY<f64>,
    a: XY<f64>,
}

// create a struct called CellGrid, which is a 2d vector of Cells
struct CellGrid {
    // TODO make this private
    w: usize,
    h: usize,
    cells: Vec<Vec<Cell>>,
    stationary_charges: Vec<StationaryCharge>,
    movable_charges: Vec<MovableCharge>,
    // movement stuff
    track_movement: bool,
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

    fn new_from_file(file: &str, save_movement: bool) -> Self {
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
    fn populate_field(&mut self) {
        for (y, row) in self.cells.iter_mut().enumerate() {
            for (x, cell) in row.iter_mut().enumerate() {
                let cell_data = field_intensity_potential(x, y, &self.stationary_charges);
                cell.e.x = cell_data.intensity.x;
                cell.e.y = cell_data.intensity.y;
                cell.v = cell_data.potential;
            }
        }
    }

    fn save_grid_to_file(&self, file: &str) {
        let mut output_file = fs::File::create(file).expect("Nie można utworzyć pliku");
        for (y, row) in self.cells.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                // format: x, y, charge, Ex, Ey, E, V
                writeln!(
                    output_file,
                    "{:.6} {:.6} {:.6} {:.6} {:.6} {:.6} {:.6}",
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
        // remove the last newline
        remove_last_char(&mut output_file);
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

    fn save_movement_history(&self) {
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
            remove_last_char(&mut output_file);
            output_file.flush().expect("Nie można wyczyścić bufora");
        }
    }

    fn add_movable_charge(&mut self, charge: MovableCharge) {
        self.movable_charges.push(charge);
        // add a new vector to the movement history
        if self.track_movement {
            self.movement_history.push(Vec::new());
        }
    }

    fn update_movable_charges(&mut self, delta_t: f64) {
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
            if intensity.x.is_infinite() {
                println!("Kolizja");
                movable_charge.collided = true;
                movable_charge.should_move = false;
                continue;
            }

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

async fn macroquad_display(cellgrid: &mut CellGrid) {
    let mut steps_by_frame = 15000;
    let mut delta_t = 0.00001;
    // TODO abstract the two above to speed and resolution

    let mut running = false;
    let mut screen_h;
    let mut screen_w;

    let mut image = Image::gen_image_color(cellgrid.w as u16, cellgrid.h as u16, BLACK);
    let texture = Texture2D::from_image(&image);
    let mut should_save = false;

    loop {
        screen_h = screen_height();
        screen_w = screen_width();

        let start = Instant::now();
        if running {
            for _ in 0..steps_by_frame {
                cellgrid.update_movable_charges(delta_t);
            }
        }
        let update_time = start.elapsed().as_micros();

        // fit the grid to the screen
        let scale_x = screen_w / (cellgrid.w as f32);
        let scale_y = screen_h / (cellgrid.h as f32);

        // display intensity
        for (y, row) in cellgrid.cells.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                let intensity = cell.e.length() as f32 * 3.;
                image.set_pixel(
                    x as u32,
                    y as u32,
                    Color::new(intensity, intensity, intensity, 1.0),
                );
            }
        }
        texture.update(&image);
        // draw stretched texture
        draw_texture_ex(
            texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_w, screen_h)),
                ..Default::default()
            },
        );

        // display stationary charges
        for charge in &cellgrid.stationary_charges {
            draw_circle(
                charge.x as f32 * scale_x + scale_x / 2.0,
                charge.y as f32 * scale_y + scale_y / 2.0,
                5.0,
                RED,
            );
        }

        // display movable charges and draw force vectors as arrows
        for charge in cellgrid.movable_charges.iter().filter(|c| c.should_move) {
            let charge_x_scaled = charge.x as f32 * scale_x + scale_x / 2.0;
            let charge_y_scaled = charge.y as f32 * scale_y + scale_y / 2.0;
            draw_circle(charge_x_scaled, charge_y_scaled, 5.0, GREEN);

            // draw force vector
            let fx = charge.m * charge.a.x * 40.0;
            let fy = charge.m * charge.a.y * 40.0;
            draw_line(
                charge_x_scaled,
                charge_y_scaled,
                charge_x_scaled + fx as f32 * scale_x,
                charge_y_scaled + fy as f32 * scale_y,
                1.0,
                YELLOW,
            );

            // draw velocity vector
            let vx = charge.v.x * 4.0;
            let vy = charge.v.y * 4.0;
            draw_line(
                charge_x_scaled,
                charge_y_scaled,
                charge_x_scaled + vx as f32 * scale_x,
                charge_y_scaled + vy as f32 * scale_y,
                1.0,
                BLUE,
            );

            // show charge values above the charge (rounded to 2 decimal places), angle in degrees
            draw_text(&format!("x: {:.2}, y: {:.2}, q: {:.2}, m: {:.2}, v: ({:.2}, {:.2} | {:.2}°), a: ({:.2}, {:.2} | {:.2}°)", charge.x, charge.y, charge.q, charge.m, charge.v.x, charge.v.y, charge.v.angle().to_degrees(), charge.a.x, charge.a.y, charge.a.angle().to_degrees()), charge_x_scaled, charge_y_scaled - 20.0, 10.0, WHITE);
        }

        // pause when Space is pressed
        if is_key_pressed(KeyCode::Space) {
            running = !running;
        }

        // we save on the next frame so that the user can see the saving message
        if should_save {
            cellgrid.save_movement_history();
            should_save = false;
        }
        // save movement when S is pressed
        if is_key_pressed(KeyCode::S) {
            draw_text("Zapisywanie ruchu...", 10.0, 50.0, 20.0, WHITE);
            should_save = true;
        }


        egui_macroquad::ui(|egui_ctx| {
            egui::Window::new("Informacje")
                .default_pos(Pos2::new(10.0, 40.0))
                .show(egui_ctx, |ui| {
                    egui::Grid::new("grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        // TODO divide into subcategories, include:
                        // charges that collided, charges that left the screen, etc.
                        ui.label("FPS");
                        ui.label(&get_fps().to_string());
                        ui.end_row();
                        ui.label("Kroki na klatke");
                        ui.label(&steps_by_frame.to_string());
                        ui.end_row();
                        ui.label("Delta t");
                        ui.label(&delta_t.to_string());
                        ui.end_row();
                        ui.label("Liczba ładunków");
                        ui.label(&cellgrid.movable_charges.len().to_string());
                        ui.end_row();
                        ui.label("Liczba ładunków stacjonarnych");
                        ui.label(&cellgrid.stationary_charges.len().to_string());
                        ui.end_row();
                        ui.label("Czas obliczeń na klatkę");
                        ui.label(&format!("{}ms", update_time as f64 / 1000.0));
                        ui.end_row();
                        ui.label("Czas renderowania");
                        ui.label(&format!("{}ms", get_frame_time() * 1000.0));
                        ui.end_row();
                    });
                });
            egui::Window::new("Ustawienia symulacji")
                .default_pos(Pos2::new(10.0, 40.0))
                .show(egui_ctx, |ui| {
                    egui::Grid::new("grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Symulacja");
                        ui.add(toggle::toggle(&mut running));
                        ui.end_row();
                        ui.label("Liczba kroków obliczeń na klatkę");
                        ui.add(egui::Slider::new(&mut steps_by_frame, 1..=100_000));
                        ui.end_row();
                        ui.label("Delta t");
                        ui.add(egui::Slider::new(&mut delta_t, 0.00001..=0.1));
                        ui.end_row();
                    })
                });
        });

        egui_macroquad::draw();


        // sliders

        // TODO:
        // - add a way to change the number of steps per frame
        // - add a way to change delta_t
        // - make a way to add charges in gui
        // - Replace XY with Vec2
        // - use egui
        // OPTIONAL:
        // - remove file operations and time for wasm build

        next_frame().await
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Nie pokazuj okna z symulacją
    #[arg(long, default_value_t = false)]
    no_gui: bool,

    /// Maksymalna liczba kroków symulacji
    #[arg(short, long, default_value_t = 10000)]
    max_steps: u32,

    /// Przyjęta delta dla symulacji
    #[arg(short, long, default_value_t = 0.01)]
    delta_t: f64,

    /// Czy symulacja powinna być przerwana po opuszczeniu siatki przez ładunek
    #[arg(long, default_value_t = false)]
    stop_on_exit: bool,

    /// Czy zapisać natężenie pola do pliku
    #[arg(long, default_value_t = false)]
    save_field: bool,

    /// Czy zapisać ruch ładunków do pliku
    #[arg(long, default_value_t = false)]
    save_movement: bool,
}

#[macroquad::main("Symulacja")]
async fn main() {
    let args = Args::parse();

    // create output directory if it doesn't exist
    if !Path::new("output").exists() {
        fs::create_dir("output").unwrap();
    }

    let mut cellgrid = CellGrid::new_from_file("ładunki.csv", args.save_movement);
    println!("Odczytane ładunki:");
    for charge in &cellgrid.stationary_charges {
        println!("x: {}, y: {}, q: {}", charge.x, charge.y, charge.q);
    }

    // calculate the field only for saving or gui background
    if args.save_field || !args.no_gui {
        let start = Instant::now();
        cellgrid.populate_field();
        let populate_time = start.elapsed().as_micros();
        // cellgrid.display_potential_color();
        println!("Czas obliczeń: {}ms", populate_time as f64 / 1000.0);

        if args.save_field {
            cellgrid.save_grid_to_file("output/output_grid.csv");
        }
    }

    let mut rng = ChaCha8Rng::seed_from_u64(0);
    // add multiple charges, coming from all directions, all places, at different speeds
    for _ in 0..5 {
        let x = rng.gen_range(0.0..cellgrid.w as f64);
        let y = rng.gen_range(0.0..cellgrid.h as f64);
        let q = rng.gen_range(-30.0..30.0);
        let m = rng.gen_range(0.0..20.0);
        let v = XY {
            x: rng.gen_range(-10.0..10.0),
            y: rng.gen_range(-10.0..10.0),
        };
        let a = XY {
            x: rng.gen_range(-10.0..10.0),
            y: rng.gen_range(-10.0..10.0),
        };
        cellgrid.add_movable_charge(MovableCharge {
            x,
            y,
            q,
            m,
            v,
            a,
            should_move: true,
            collided: false,
        });
    }
    println!();

    if args.no_gui {
        println!("Symulowanie przez max. {} kroków", args.max_steps);

        // simulation
        let start = Instant::now();
        if args.stop_on_exit {
            'simulation: for _ in 0..args.max_steps {
                cellgrid.update_movable_charges(args.delta_t);
                for (i, charge) in cellgrid.movable_charges.iter().enumerate() {
                    if charge.x < 0.0
                        || charge.x > cellgrid.w as f64
                        || charge.y < 0.0
                        || charge.y > cellgrid.h as f64
                    {
                        println!("Ładunek {} opuścił siatkę", i);
                        break 'simulation;
                    }
                }
            }
        } else {
            for _ in 0..args.max_steps {
                cellgrid.update_movable_charges(args.delta_t);
            }
        }
        let update_time = start.elapsed().as_micros();
        println!("Czas obliczeń: {}ms", update_time as f64 / 1000.0);

        // saving movement history to file
        println!("Zapisywanie ruchu do pliku");
        let start = Instant::now();
        cellgrid.save_movement_history();
        let save_time = start.elapsed().as_micros();
        println!("Czas zapisu: {}ms", save_time as f64 / 1000.0);
    } else {
        macroquad_display(&mut cellgrid).await;
    }
}
