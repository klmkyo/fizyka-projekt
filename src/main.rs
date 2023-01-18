use clap::Parser;
use egui::Pos2;
use lib::toggle;
use macroquad::{
    self,
    prelude::{*},
};
use rand_chacha::ChaCha8Rng;
use std::{
    fs,
    path::Path,
    time::Instant,
};
extern crate rand;
use colored::Colorize;
use rand::{Rng, SeedableRng};

pub mod cellgrid;
use cellgrid::*;
pub mod movable_charge;
use movable_charge::*;

pub mod lib;
use crate::lib::helpers::{in_bounds, XY};

enum MouseCharge {
    Positive,
    Negative,
}

// UI main loop
async fn macroquad_display(cellgrid: &mut CellGrid) {
    let mut steps_by_frame = 1000;
    let mut delta_t = 0.00000001;
    // TODO abstract the two above to speed and resolution
    let mut running = false;

    let mut draw_details = true;
    let mut draw_vectors = true;
    let mut mouse_charge = MouseCharge::Positive;

    let (cellgrid_w, cellgrid_h) = cellgrid.get_dimensions();
    let mut image = Image::gen_image_color(cellgrid_w as u16, cellgrid_h as u16, BLACK);
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

        let (cellgrid_w, cellgrid_h) = cellgrid.get_dimensions();
        // fit the grid to the screen
        let scale_x = screen_w / (cellgrid_w as f32);
        let scale_y = screen_h / (cellgrid_h as f32);

        let (mouse_x, mouse_y) = mouse_position();
        let mouse_x_scaled: f64 = (mouse_x / scale_x).into();
        let mouse_y_scaled: f64 = (mouse_y / scale_y).into();

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
                const ACCELERATION_VECTOR_SCALE: f32 = 2.5 * 10e2;
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
                const VELOCITY_VECTOR_SCALE: f32 = 1. * 10e1;
                draw_line(
                    charge_x_scaled,
                    charge_y_scaled,
                    charge_x_scaled + vx as f32 * scale_x / VELOCITY_VECTOR_SCALE,
                    charge_y_scaled + vy as f32 * scale_y / VELOCITY_VECTOR_SCALE,
                    1.0,
                    BLUE,
                );
            }

            if draw_details {
                // show charge values above the charge (rounded to 2 decimal places), angle in degrees
                draw_text(&format!("x: {:.2}, y: {:.2}, q: {:.2}, m: {:.2}, v: ({:.2}, {:.2} | {:.2}°), a: ({:.2}, {:.2} | {:.2}°)", charge.x, charge.y, charge.q, charge.m, charge.v.x, charge.v.y, charge.v.angle().to_degrees(), charge.a.x, charge.a.y, charge.a.angle().to_degrees()), charge_x_scaled, charge_y_scaled - 20.0, 10.0, WHITE);
            }
        }

        // draw intensity vector at user's mouse position
        if draw_vectors {
            let intensity = field_intensity_movable(
                mouse_x_scaled,
                mouse_y_scaled,
                &cellgrid.stationary_charges,
            )
            .unwrap_or(XY { x: 0., y: 0. });

            const INTENSITY_VECTOR_SCALE: f32 = 1. * 10e5;

            let mut end_mouse = XY {
                x: mouse_x,
                y: mouse_y,
            };
            match mouse_charge {
                MouseCharge::Positive => {
                    end_mouse.x += intensity.x as f32 / INTENSITY_VECTOR_SCALE;
                    end_mouse.y += intensity.y as f32 / INTENSITY_VECTOR_SCALE;
                }
                MouseCharge::Negative => {
                    end_mouse.x -= intensity.x as f32 / INTENSITY_VECTOR_SCALE;
                    end_mouse.y -= intensity.y as f32 / INTENSITY_VECTOR_SCALE;
                }
            }

            draw_line(
                mouse_x,
                mouse_y,
                end_mouse.x,
                end_mouse.y,
                1.0,
                match mouse_charge {
                    MouseCharge::Positive => RED,
                    MouseCharge::Negative => BLUE,
                },
            );
        }

        if draw_details {
             // print the field intensity at mouse position
             let intensity = field_intensity_movable(
                mouse_x_scaled,
                mouse_y_scaled,
                &cellgrid.stationary_charges,
            )
            .unwrap_or(XY { x: 0., y: 0. });

            draw_text(
                &format!("E: ({:.2}, {:.2} | {:.2}°)", intensity.x, intensity.y, intensity.angle().to_degrees()),
                mouse_x as f32 + 10.0,
                mouse_y as f32 - 10.0,
                10.0,
                WHITE,
            );
        }

        // pause when Space is pressed
        if is_key_pressed(KeyCode::Space) {
            running = !running;
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
                                .to_owned()
                                + "s";
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
                            ui.label(stringified_delta_t.to_string());
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
                            ui.add(toggle::toggle(&mut draw_details));
                            ui.end_row();
                            ui.label("Pokaż wektory");
                            ui.add(toggle::toggle(&mut draw_vectors));
                            ui.end_row();
                        })
                });
        });

        // on click invert the charge
        if is_mouse_button_pressed(MouseButton::Left) {
            mouse_charge = match mouse_charge {
                MouseCharge::Positive => MouseCharge::Negative,
                MouseCharge::Negative => MouseCharge::Positive,
            };
        }

        egui_macroquad::draw();

        // sliders

        // TODO:
        // - add a way to change the number of steps per frame
        // - add a way to change delta_t
        // - make a way to add charges in gui
        // - Replace XY with Vec2
        // - maybe migrate everything to egui
        // - save preferences to a file
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
    bez_gui: bool,

    /// Maksymalna liczba kroków symulacji
    #[arg(short, long, default_value_t = 10000)]
    max_krokow: u32,

    /// Przyjęta delta dla symulacji
    #[arg(short, long, default_value_t = 0.000001)]
    delta_t: f64,

    /// Czy symulacja powinna być przerwana gdy wszystkie ładunki opuszczą siatkę
    #[arg(long, default_value_t = false)]
    zakoncz_po_opuszczeniu: bool,

    /// Czy zapisać natężenie pola do pliku
    #[arg(long, default_value_t = false)]
    zapisz_pole: bool,

    /// Czy zapisać ruch ładunków do pliku (działa tylko bez GUI)
    #[arg(long, default_value_t = false)]
    zapisz_ruch: bool,
}

#[macroquad::main("Symulacja")]
async fn main() {
    let args = Args::parse();

    // create output directory if it doesn't exist
    if !Path::new("output").exists() {
        fs::create_dir("output").unwrap();
    }

    // read charges from file
    let mut cellgrid = CellGrid::new_from_file("ładunki_stacjonarne.csv", args.zapisz_ruch);
    println!("Odczytane ładunki:");
    for charge in &cellgrid.stationary_charges {
        println!("x: {}, y: {}, q: {}", charge.x, charge.y, charge.q);
    }

    // calculate the field only for saving or gui background
    if args.zapisz_pole || !args.bez_gui {
        let start = Instant::now();
        cellgrid.populate_field();
        let populate_time = start.elapsed().as_micros();
        // cellgrid.display_potential_color();
        println!("Czas obliczeń: {}ms", populate_time as f64 / 1000.0);

        if args.zapisz_pole {
            cellgrid.save_grid_to_file("output/output_grid.csv");
        }
    }

    // uncomment for fixed seed
    // let mut rng = ChaCha8Rng::seed_from_u64(0);
    // let mut rng = rand::thread_rng();
    // add multiple charges, coming from all directions, all places, at different speeds

    // 10^(-4)
    // const CHARGE_SCALE: f64 = 0.0001;
    // for _ in 0..50 {
    //     let (cellgrid_width, cellgrid_height) = cellgrid.get_dimensions();
    //     let x = rng.gen_range(0.0..cellgrid_width as f64);
    //     let y = rng.gen_range(0.0..cellgrid_height as f64);
    //     let q = rng.gen_range(-30.0 * CHARGE_SCALE ..30.0 * CHARGE_SCALE);
    //     let m = rng.gen_range(0.0..20.0);
    //     let v = XY {
    //         x: rng.gen_range(-10.0..10.0),
    //         y: rng.gen_range(-10.0..10.0),
    //     };
    //     let a = XY {
    //         x: rng.gen_range(-10.0..10.0),
    //         y: rng.gen_range(-10.0..10.0),
    //     };
    //     cellgrid.add_movable_charge(MovableCharge {
    //         x,
    //         y,
    //         q,
    //         m,
    //         v,
    //         a,
    //         should_move: true,
    //         collided: false,
    //     });
    // }

    let movable_charges = MovableCharge::vec_from_file("ładunki_ruchome.csv");
    for charge in movable_charges {
        cellgrid.add_movable_charge(charge);
    }

    println!();

    if args.bez_gui {
        // if there is neither save_field nor save_movement, just exit
        if !args.zapisz_pole && !args.zapisz_ruch {
            println!("Wybrano tryb bez interfejsu graficznego, ale nie wybrano żadnej z opcji zapisu! (wyniki nie zostaną zapisane)");
            println!(
                "Aby zapisać pole, użyj opcji {}",
                "--zapisz-pole".to_string().bold()
            );
            println!(
                "Aby zapisać ruch ładunków, użyj opcji {}",
                "--zapisz-ruch".to_string().bold()
            );
            return;
        }

        if args.zapisz_pole {
            println!("Zapisano pole do pliku output_grid.csv");
        }

        if !args.zapisz_ruch {
            return;
        }
        println!("Symulowanie przez max. {} kroków", args.max_krokow);

        // simulation
        let start = Instant::now();
        if args.zakoncz_po_opuszczeniu {
            let (cellgrid_w, cellgrid_h) = cellgrid.get_dimensions();
            let cellgrid_w_f64 = cellgrid_w as f64;
            let cellgrid_h_f64 = cellgrid_h as f64;

            'simulation: for _ in 0..args.max_krokow {
                cellgrid.update_movable_charges(args.delta_t);

                for charge in cellgrid.movable_charges.iter() {
                    if in_bounds(charge.x, charge.y, 0., cellgrid_w_f64, 0., cellgrid_h_f64) {
                        // if there is at least one charge inside
                        continue 'simulation;
                    }
                }
                // code reachable only if all charges are out of bounds
                println!("Wszystkie ładunki opuściły siatkę");
                break 'simulation;
            }
        } else {
            for _ in 0..args.max_krokow {
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
        if args.zapisz_ruch {
            eprintln!("Opcja {} nie jest obsługiwana w trybie graficznym!", "--zapisz-ruch".to_string().bold());
            eprintln!("Dodaj flagę {} aby wyłączyć interfejs graficzny, lub wyłącz zapisywanie ruchu", "--bez-gui".to_string().bold());
            return;
        }

        // display gui
        macroquad_display(&mut cellgrid).await;
    }
}
