use clap::Parser;
use egui::Pos2;
use lib::toggle;
use macroquad::{self, prelude::*};
use rand_chacha::ChaCha8Rng;
use std::{fs, path::Path, time::Instant, cell, f32::INFINITY};
extern crate rand;
use colored::Colorize;
use rand::{Rng, SeedableRng};

pub mod cellgrid;
use cellgrid::*;
pub mod movable_charge;
use movable_charge::*;

pub mod lib;
use crate::lib::helpers::{ensure_files_exist, in_bounds, XY};

enum MouseCharge {
    Positive,
    Negative,
}

fn fill_texture_with_intensity(
    potential_mode: bool,
    stationary_charges: &Vec<StationaryCharge>,
    intensity_percentile: f64,
    potential_percentile: f64,
    cellgrid_w: usize,
    cellgrid_h: usize,
    screen_w: f32,
    screen_h: f32,
) -> Texture2D {
    let mut image = Image::gen_image_color(screen_w as u16, screen_h as u16, BLACK);
    // display intensity
    for y in 0..screen_h as u32 {
        for x in 0..screen_w as u32 {
            let virtual_x = (x as f64 / screen_w as f64) * cellgrid_w as f64;
            let virtual_y = (y as f64 / screen_h as f64) * cellgrid_h as f64;
            let intensity_potential_option = field_intensity_potential(virtual_x, virtual_y, stationary_charges);

            let (intensity, potential) = match intensity_potential_option {
                Some((intensity, potential)) => (intensity, potential),
                None => (f64::INFINITY, f64::INFINITY),
            };

            if potential_mode {
                // if potential is greater than 0, then the charge is positive, so the color should be red
                // if potential is less than 0, then the charge is negative, so the color should be blue
                let saturation = (potential.abs() / potential_percentile) as f32;
                let color = if potential > 0. {
                    // red
                    Color::new(saturation, 0., 0., 1.0)
                } else {
                    // blue
                    Color::new(0., 0., saturation, 1.0)
                };
                image.set_pixel(
                    x as u32,
                    y as u32,
                    color,
                );
            }
            else {
                let intensity = 100. * (intensity.abs() / intensity_percentile) as f32;
                image.set_pixel(
                    x as u32,
                    y as u32,
                    Color::new(intensity, intensity, intensity, 1.0),
                );
            }
        }
    }

    // let intensity = 1. * (cell.e.length() / field_intenity_percentile) as f32;
    // image.set_pixel(
    //     x as u32,
    //     y as u32,
    //     Color::new(intensity, intensity, intensity, 1.0),
    // );

    Texture2D::from_image(&image)
}

// UI main loop
async fn macroquad_display(cellgrid: &mut CellGrid, delta_t: f64) {
    let mut steps_by_frame = 1;
    let mut delta_t = delta_t;
    // TODO abstract the two above to speed and resolution
    let mut running = false;

    let mut draw_details = true;
    let mut draw_vectors = true;
    let mut mouse_charge = MouseCharge::Positive;
    let mut potential_display_mode = true;
    let mut percentile = 0.95;

    let mut old_potential_display_mode = potential_display_mode;
    let mut old_percentile = percentile;

    let (cellgrid_w, cellgrid_h) = cellgrid.get_dimensions();
    let mut screen_h = screen_height();
    let mut screen_w = screen_width();
    let mut texture = Texture2D::empty();

    let mut time_elapsed: f64 = 0.;

    let (mut intensity_percentile, mut potential_percentile) = cellgrid.field_percentiles(percentile);

    let velocity_vector_scale: f32 = 3. * 10e2;
    let acceleration_vector_scale: f64 = 1.8 * 10e6;
    let intensity_vector_scale: f32 = 4. * 10e-5;

    // println!("field_intenity_percentile: {}", field_intenity_percentile);

    texture = fill_texture_with_intensity(potential_display_mode, &cellgrid.stationary_charges, intensity_percentile, potential_percentile, cellgrid_w, cellgrid_h, screen_w, screen_h);

    loop {
        let (new_screen_w, new_screen_h) = (screen_width(), screen_height());

        if new_screen_w != screen_w || new_screen_h != screen_h || potential_display_mode != old_potential_display_mode || percentile != old_percentile {
            (intensity_percentile, potential_percentile) = cellgrid.field_percentiles(percentile);
            texture = fill_texture_with_intensity(potential_display_mode, &cellgrid.stationary_charges, intensity_percentile, potential_percentile, cellgrid_w, cellgrid_h, screen_w, screen_h);
            (screen_w, screen_h) = (new_screen_w, new_screen_h);
            old_potential_display_mode = potential_display_mode;
            old_percentile = percentile;
        }

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
                draw_line(
                    charge_x_scaled,
                    charge_y_scaled,
                    charge_x_scaled + ((charge.a.x / acceleration_vector_scale) as f32 * scale_x),
                    charge_y_scaled + ((charge.a.y / acceleration_vector_scale) as f32 * scale_y),
                    1.0,
                    YELLOW,
                );

                // draw velocity vector
                let vx = charge.v.x;
                let vy = charge.v.y;
                draw_line(
                    charge_x_scaled,
                    charge_y_scaled,
                    charge_x_scaled + vx as f32 * scale_x / velocity_vector_scale,
                    charge_y_scaled + vy as f32 * scale_y / velocity_vector_scale,
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


            let mut end_mouse = XY {
                x: mouse_x,
                y: mouse_y,
            };
            match mouse_charge {
                MouseCharge::Positive => {
                    end_mouse.x += intensity.x as f32 / intensity_vector_scale;
                    end_mouse.y += intensity.y as f32 / intensity_vector_scale;
                }
                MouseCharge::Negative => {
                    end_mouse.x -= intensity.x as f32 / intensity_vector_scale;
                    end_mouse.y -= intensity.y as f32 / intensity_vector_scale;
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

        // if draw_details {
        //     // print the field intensity at mouse position
        //     let (intensity, potential) = field_intensity_potential(mouse_x_scaled, mouse_y_scaled, &cellgrid.stationary_charges).unwrap_or((0.,0.));
        //     draw_text(&format!("E: {:.10}, V: {:.10}", intensity, potential), mouse_x, mouse_y - 20.0, 10.0, WHITE);
        // }

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
                            ui.label("Tło: ".to_owned() + if potential_display_mode {"potencjał"} else {"natężenie pola"});
                            ui.add(toggle::toggle(&mut potential_display_mode));
                            ui.end_row();
                            ui.label("Percentyl tła");
                            ui.add(egui::Slider::new(&mut percentile, 0.0..=0.999).text(""));
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

    /// Przyjęta delta dla symulacji
    #[arg(short, long, default_value_t = 0.000001)]
    delta_t: f64,

    /// (bez GUI) Czy symulacja powinna być przerwana gdy wszystkie ładunki opuszczą siatkę
    #[arg(long, default_value_t = false)]
    zakoncz_po_opuszczeniu: bool,

    /// (bez GUI) Czy zapisać natężenie pola do pliku
    #[arg(long, default_value_t = false)]
    zapisz_pole: bool,

    /// (bez GUI) Czy zapisać ruch ładunków do pliku
    #[arg(long, default_value_t = false)]
    zapisz_ruch: bool,

    /// (bez GUI) Maksymalna liczba kroków symulacji
    #[arg(short, long, default_value_t = 10000)]
    max_krokow: u32,
}

#[macroquad::main("Symulacja")]
async fn main() {
    let args = Args::parse();

    ensure_files_exist();

    // read charges from file
    let mut cellgrid = CellGrid::new_from_file("ladunki_stacjonarne.txt", args.zapisz_ruch);

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

    let movable_charges = MovableCharge::vec_from_file("ladunki_ruchome.txt");
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
            eprintln!(
                "Opcja {} nie jest obsługiwana w trybie graficznym!",
                "--zapisz-ruch".to_string().bold()
            );
            eprintln!(
                "Dodaj flagę {} aby wyłączyć interfejs graficzny, lub wyłącz zapisywanie ruchu",
                "--bez-gui".to_string().bold()
            );
            return;
        }

        // display gui
        macroquad_display(&mut cellgrid, args.delta_t).await;
    }
}
