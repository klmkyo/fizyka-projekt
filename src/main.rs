use clap::Parser;
use egui::Pos2;
use macroquad::{self, prelude::*};
use std::{
    f64::INFINITY,
    fs,
    io::{BufWriter, Seek, SeekFrom, Write},
    path::Path,
    str::FromStr,
    time::Instant, cell,
};
extern crate rand;
use rand::{Rng};

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

// function used to calculate the field intensity and potential generated by
// stationary charges at a given point (x, y)
// used for displaying the background, and for calculating the field intensity
// which is then saved to a file
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
            (stationary_charge.q * (x as i32 - stationary_charge.x as i32) as f64) / (r.powi(2));
        intensity.y +=
            (stationary_charge.q * (y as i32 - stationary_charge.y as i32) as f64) / (r.powi(2));
        potential += stationary_charge.q / r;
    }
    CellData {
        intensity,
        potential,
    }
}

const K: f64 = 8.99e9;

// static mut lowest: f64 = INFINITY;

// This function calculates the field intensity at a point (x, y) caused by a
// set of stationary charges. The function returns an XY struct containing the
// field intnsity for x and y axis.
fn field_intensity_movable(x: f64, y: f64, stationary_charges: &Vec<StationaryCharge>) -> XY<f64> {
    let mut intensity_xy = XY { x: 0.0, y: 0.0 };
    for stationary_charge in stationary_charges {
        let r_sq =
            (x - stationary_charge.x as f64).powi(2) + (y - stationary_charge.y as f64).powi(2);
        let r = r_sq.sqrt();

        // If the distance between the given point and the stationary charge is
        // less than 2, the field intensity is goes way too high for accurate calculations.
        // We return infinity in this case, which later on is interpreted as a
        // collision of charges.
        if r < 2. {
            return XY {
                x: INFINITY,
                y: INFINITY,
            };
        }

        // code for debugging
        // if r < unsafe { lowest } {
        //     unsafe {
        //         lowest = r;
        //     }
        //     println!("lowest: {}", unsafe { lowest });
        // }

        // The electric field intensity (E) at a point caused by a stationary
        // charge is given by the formula E = k * q / r^2, where k is a constant,
        // q is the charge of the stationary charge, and r is the distance to the
        // point. We calculate the intensity of the field at the given point
        // caused by the given stationary charge and add it to the total intensity
        // vector.
        let intensity_times_k = K * stationary_charge.q / r_sq;

        intensity_xy.x += intensity_times_k * (x - stationary_charge.x as f64) / r;
        intensity_xy.y += intensity_times_k * (y - stationary_charge.y as f64) / r;

        // another way to calculate the intensity vector
        // get the angle of the intensity vector
        // let angle = (y - stationary_charge.y as f64).atan2(x - stationary_charge.x as f64);
        // calculate the intensity vector using trigonometry
        // let result2 = XY {
        //     x: intensity * angle.cos(),
        //     y: intensity * angle.sin(),
        // };
    }
    intensity_xy
}

#[inline(always)]
fn in_bounds<T: PartialOrd>(x: T, y: T, min_x: T, max_x: T, min_y: T, max_y: T) -> bool {
    x > min_x && x < max_x && y > min_y && y < max_y
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

// MovementStep is used to track the movement of movable charges, so that they can be saved to a file (if user wants to)
struct MovementStep {
    x: f64,
    y: f64,
    v: XY<f64>,
    a: XY<f64>,
}

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
        self.movement_history.push(Vec::new());
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


// UI main loop
async fn macroquad_display(cellgrid: &mut CellGrid) {            
    let mut steps_by_frame = 1000;
    let mut delta_t = 0.00000001;
    // TODO abstract the two above to speed and resolution

    let mut running = false;

    let mut save_movement_next_frame = false;
    let mut charge_details = true;
    let mut draw_vectors = true;

    let mut image = Image::gen_image_color(cellgrid.w as u16, cellgrid.h as u16, BLACK);
    let texture = Texture2D::from_image(&image);
    let mut screen_h;
    let mut screen_w;

    let mut time_elapsed: f64 = 0.;

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

    // display stationary charges
    for charge in &cellgrid.stationary_charges {
        let x = charge.x as u32;
        let y = charge.y as u32;
        let color = if charge.q > 0. { RED } else { BLUE };
        image.set_pixel(x, y, color);
    }

    texture.update(&image);

    loop {
        screen_h = screen_height();
        screen_w = screen_width();

        let start = Instant::now();
        if running {
            for _ in 0..steps_by_frame {
                cellgrid.update_movable_charges(delta_t);
            }
            time_elapsed += delta_t * steps_by_frame as f64;
        }
        let update_time = start.elapsed().as_micros();

        // fit the grid to the screen
        let scale_x = screen_w / (cellgrid.w as f32);
        let scale_y = screen_h / (cellgrid.h as f32);

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
        // for charge in &cellgrid.stationary_charges {
        //     draw_circle(
        //         charge.x as f32 * scale_x + scale_x / 2.0,
        //         charge.y as f32 * scale_y + scale_y / 2.0,
        //         5.0,
        //         RED,
        //     );
        // }

        // display movable charges and draw force vectors as arrows
        for charge in cellgrid.movable_charges.iter().filter(|c| c.should_move) {
            let charge_x_scaled = charge.x as f32 * scale_x + scale_x / 2.0;
            let charge_y_scaled = charge.y as f32 * scale_y + scale_y / 2.0;

            // draw blue or red circle depending on charge
            // radius depends on mass * charge, should range from 2 to 10
            draw_circle(
                charge_x_scaled,
                charge_y_scaled,
                3.5 + (charge.m * charge.q / 200.0) as f32,
                if charge.q > 0. { RED } else { BLUE },
            );


            if draw_vectors {
                // draw acceleration vector
                const ACCELERATION_VECTOR_SCALE: f32 = 5. * 10e5;
                draw_line(
                    charge_x_scaled,
                    charge_y_scaled,
                    charge_x_scaled + charge.a.x as f32 * scale_x / ACCELERATION_VECTOR_SCALE,
                    charge_y_scaled + charge.a.y as f32 * scale_y / ACCELERATION_VECTOR_SCALE,
                    1.0,
                    YELLOW,
                );

                // draw velocity vector
                let vx = charge.v.x;
                let vy = charge.v.y;
                const VELOCITY_VECTOR_SCALE: f32 = 1. * 10e3;
                draw_line(
                    charge_x_scaled,
                    charge_y_scaled,
                    charge_x_scaled + vx as f32 * scale_x / VELOCITY_VECTOR_SCALE,
                    charge_y_scaled + vy as f32 * scale_y / VELOCITY_VECTOR_SCALE,
                    1.0,
                    BLUE,
                );
            }

            // show charge values above the charge (rounded to 2 decimal places), angle in degrees
            if charge_details {
                draw_text(&format!("x: {:.2}, y: {:.2}, q: {:.2}, m: {:.2}, v: ({:.2}, {:.2} | {:.2}°), a: ({:.2}, {:.2} | {:.2}°)", charge.x, charge.y, charge.q, charge.m, charge.v.x, charge.v.y, charge.v.angle().to_degrees(), charge.a.x, charge.a.y, charge.a.angle().to_degrees()), charge_x_scaled, charge_y_scaled - 20.0, 10.0, WHITE);
            }
        }

        // draw intensity vector at user's mouse position
        if draw_vectors {
            let mouse_x = mouse_position().0;
            let mouse_y = mouse_position().1;
            let mouse_x_scaled: f64 = (mouse_x / scale_x).into();
            let mouse_y_scaled: f64 = (mouse_y / scale_y).into();

            let force = field_intensity_movable(mouse_x_scaled, mouse_y_scaled, &cellgrid.stationary_charges);
            const FORCE_VECTOR_SCALE: f32 = 1. * 10e5;
            draw_line(
                mouse_x,
                mouse_y,
                mouse_x + force.x as f32 / FORCE_VECTOR_SCALE,
                mouse_y + force.y as f32 / FORCE_VECTOR_SCALE,
                1.0,
                GREEN,
            );
        }

        // pause when Space is pressed
        if is_key_pressed(KeyCode::Space) {
            running = !running;
        }

        // we save on the next frame so that the user can see the saving message
        if save_movement_next_frame {
            cellgrid.save_movement_history();
            save_movement_next_frame = false;
        }
        // save movement when S is pressed
        if is_key_pressed(KeyCode::S) {
            draw_text("Zapisywanie ruchu...", 10.0, 50.0, 20.0, WHITE);
            save_movement_next_frame = true;
        }

        egui_macroquad::ui(|egui_ctx| {
            let info_window = egui::Window::new("Informacje")
                .default_pos(Pos2::new(10.0, 40.0))
                .resizable(false)
                .show(egui_ctx, |ui| {
                    egui::Grid::new("grid")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .min_col_width(100.0)
                        .striped(true)
                        .show(ui, |ui| {
                            // TODO divide into subcategories, include:
                            // charges that collided, charges that left the screen, etc.
                            ui.label("Upłynięty czas symulacji");
                            // print only the necessary zeros
                            let stringified_time = format!("{:.16}", time_elapsed);
                            let stringified_time = stringified_time
                                .trim_end_matches('0')
                                .trim_end_matches('.')
                                .to_owned() + "s";
                            ui.label(stringified_time);
                            ui.end_row();
                            ui.label("FPS");
                            ui.label(&get_fps().to_string());
                            ui.end_row();
                            ui.label("Kroki na klatke");
                            ui.label(&steps_by_frame.to_string());
                            ui.end_row();
                            ui.label("Delta T na krok");
                            ui.label(&delta_t.to_string());
                            ui.end_row();
                            ui.label("Delta T na klatkę");
                            let stringified_delta_t =
                                format!("{:.16}", delta_t * steps_by_frame as f64);
                            // strip trailing zeros
                            let stringified_delta_t = stringified_delta_t
                                .trim_end_matches('0')
                                .trim_end_matches('.');
                            ui.label(&format!("{}", stringified_delta_t));
                            ui.end_row();
                            ui.label("Liczba ładunków ruchomych");
                            ui.label(&cellgrid.movable_charges.len().to_string());
                            ui.end_row();
                            ui.label("Liczba kolizji");
                            ui.label(
                                &cellgrid
                                    .movable_charges
                                    .iter()
                                    .filter(|&x| (x).collided)
                                    .count()
                                    .to_string(),
                            );
                            ui.end_row();
                            ui.label("Liczba ładunków stacjonarnych");
                            ui.label(&cellgrid.stationary_charges.len().to_string());
                            ui.end_row();
                            ui.label("Czas obliczeń na klatkę");
                            ui.label(&format!("{}ms", update_time as f64 / 1000.0));
                            ui.end_row();
                            ui.label("Czas renderowania");
                            ui.label(&format!("{:.2}ms", get_frame_time() * 1000.0));
                            ui.end_row();
                        });
                });
            // place this window under the info window
            let info_rect = info_window.unwrap().response.rect;
            let info_window_pos = info_rect.min;
            egui::Window::new("Ustawienia symulacji")
                .default_pos(Pos2::new(
                    info_window_pos.x,
                    info_window_pos.y + info_rect.height() + 10.0,
                ))
                .resizable(false)
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
                            ui.add(egui::DragValue::new(&mut steps_by_frame).speed(1.0));
                            ui.end_row();
                            ui.label("Delta T na krok");
                            ui.add(egui::DragValue::new(&mut delta_t).speed(0.01));
                            ui.end_row();
                            ui.label("Informacje o ładunkach");
                            ui.add(toggle::toggle(&mut charge_details));
                            ui.end_row();
                            ui.label("Pokaż wektory");
                            ui.add(toggle::toggle(&mut draw_vectors));
                            ui.end_row();

                            // saving to a file
                            ui.label("Zapisywanie ruchu");
                            ui.add(toggle::toggle(&mut cellgrid.track_movement));
                            ui.end_row();

                            if cellgrid.track_movement {
                                ui.label("Zapisz ruch");
                                let button = egui::Button::new("Zapisz ruch");
                                if ui.add_enabled(save_movement_next_frame, button).clicked() {
                                    save_movement_next_frame = true;
                                };
                                ui.end_row();
                            }
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
        // - maybe migrate everything to egui
        // OPTIONAL:
        // - remove file operations and time for wasm build

        next_frame().await
    }
}

#[derive(Parser, Debug)]
#[command(author = "Marcin Klimek", version = "1.0", about = "Program symulujący ruch naładowanej cząsteczki w polu elektrycznym", long_about = None)]
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

    // read charges from file
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

    // uncomment for fixed seed
    // let mut rng = ChaCha8Rng::seed_from_u64(0);
    let mut rng = rand::thread_rng();
    // add multiple charges, coming from all directions, all places, at different speeds
    for _ in 0..read_input("Ile ładunków dodac?") {
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
            let cellgrid_w_f64 = cellgrid.w as f64;
            let cellgrid_h_f64 = cellgrid.h as f64;

            'simulation: for _ in 0..args.max_steps {
                cellgrid.update_movable_charges(args.delta_t);
               
                for charge in cellgrid.movable_charges.iter() {
                    if in_bounds(charge.x, charge.y, 0., cellgrid_w_f64, 0., cellgrid_h_f64)
                    {
                        // if there is at least one charge inside
                        continue 'simulation;
                    }
                }
                // code reachable only if all charges are out of bounds
                println!("Wszystkie ładunki opuściły siatkę");
                break 'simulation;
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
        // display gui
        macroquad_display(&mut cellgrid).await;
    }
}
